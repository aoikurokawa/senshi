use std::{path::PathBuf, time::Duration};

use anyhow::{anyhow, Result};
use indicatif::{ProgressBar, ProgressStyle};
use light_client::rpc::{LightClient, Rpc};
use light_compressed_token_sdk::{
    compressed_token::batch_compress::{
        create_batch_compress_instruction, BatchCompressInputs, Recipient,
    },
    spl_interface::derive_spl_interface_pda,
};
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};

use crate::{checkpoint::Checkpoint, recipients::Recipient as CliRecipient};

const MAX_ATTEMPTS: u32 = 3;

/// Distribute `recipients` of `mint` as compressed tokens, `batch_size` per
/// transaction, resuming from `checkpoint_path` if it exists.
pub async fn run(
    client: &mut LightClient,
    payer: &Keypair,
    mint: Pubkey,
    source_token_account: Pubkey,
    recipients: Vec<CliRecipient>,
    batch_size: usize,
    checkpoint_path: PathBuf,
) -> Result<()> {
    let authority = payer.pubkey();
    let token_program = spl_token::id();

    // Token pool PDA (SPL interface) for this mint — must already exist
    // (created by `create-mint`, or the TS `createTokenPool`).
    let pool = derive_spl_interface_pda(&mint, 0, false);

    let mut checkpoint = Checkpoint::load(checkpoint_path)?;

    let batches: Vec<&[CliRecipient]> = recipients.chunks(batch_size).collect();
    let total_recipients = recipients.len();

    let pb = ProgressBar::new(total_recipients as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner} [{elapsed_precise}] {bar:40} {pos}/{len} recipients ({msg})",
        )
        .unwrap(),
    );

    let mut sent = 0usize;
    for (i, chunk) in batches.iter().enumerate() {
        if checkpoint.is_done(i) {
            pb.inc(chunk.len() as u64);
            continue;
        }

        // A fresh state tree per batch spreads writes across trees.
        // For V2 batched trees new state is inserted into the output *queue*, not
        // the merkle tree — `get_output_pubkey()` returns the right one per type.
        let merkle_tree = client
            .get_random_state_tree_info()
            .map_err(|e| anyhow!("no state tree available from RPC: {e}"))?
            .get_output_pubkey()
            .map_err(|e| anyhow!("unsupported state tree type: {e}"))?;

        let batch_recipients: Vec<Recipient> = chunk
            .iter()
            .map(|r| Recipient {
                pubkey: r.address,
                amount: r.amount,
            })
            .collect();

        let inputs = BatchCompressInputs {
            fee_payer: authority,
            authority,
            spl_interface_pda: pool.pubkey,
            sender_token_account: source_token_account,
            token_program,
            merkle_tree,
            recipients: batch_recipients,
            lamports: None,
            token_pool_index: 0,
            token_pool_bump: pool.bump,
            sol_pool_pda: None,
        };

        let batch_ix = create_batch_compress_instruction(inputs)
            .map_err(|e| anyhow!("building batch_compress instruction: {e:?}"))?;

        // batch_compress with many recipients exceeds the 200k default CU limit.
        let instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
            batch_ix,
        ];

        pb.set_message(format!("batch {}/{}", i + 1, batches.len()));

        let mut attempt = 0;
        let signature = loop {
            attempt += 1;
            match client
                .create_and_send_transaction(&instructions, &authority, &[payer])
                .await
            {
                Ok(sig) => break sig,
                Err(e) if attempt < MAX_ATTEMPTS => {
                    pb.set_message(format!("batch {} retry {attempt}: {e}", i + 1));
                    tokio::time::sleep(Duration::from_millis(500 * attempt as u64)).await;
                }
                Err(e) => {
                    pb.abandon_with_message(format!("batch {} failed", i + 1));
                    return Err(anyhow!(
                        "batch {} failed after {attempt} attempts: {e}. \
                         Re-run with the same --checkpoint to resume.",
                        i + 1
                    ));
                }
            }
        };

        checkpoint.mark_done(i, signature.to_string())?;
        sent += chunk.len();
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("done");
    println!(
        "✅ airdrop complete: {} recipients across {} batches ({} newly sent)",
        total_recipients,
        batches.len(),
        sent
    );
    Ok(())
}
