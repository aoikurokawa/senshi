use anyhow::{anyhow, bail, Result};
use light_account::PackedAccounts;
use light_client::{
    indexer::{GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer},
    rpc::{LightClient, Rpc},
};
use light_compressed_account::compressed_account::PackedMerkleContext;
use light_compressed_token_sdk::{
    compressed_token::{
        transfer2::{
            create_transfer2_instruction, Transfer2AccountsMetaConfig, Transfer2Config,
            Transfer2Inputs,
        },
        CTokenAccount2,
    },
    spl_interface::derive_spl_interface_pda,
};
use light_token_interface::instructions::transfer2::MultiInputTokenDataWithContext;
use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
};

use crate::recipients::ui_to_base;

/// Decompress compressed tokens back into a normal SPL associated token account,
/// making them visible in any standard wallet.
///
/// This *spends* the owner's compressed accounts, so it fetches a validity proof
/// and packs the inputs into a `transfer2` decompress instruction.
pub async fn run(
    client: &mut LightClient,
    payer: &Keypair,
    mint: Pubkey,
    amount_ui: Option<f64>,
    decimals: u8,
    account_version: u8,
) -> Result<()> {
    let owner = payer.pubkey();
    let token_program = spl_token::id();
    let pool = derive_spl_interface_pda(&mint, 0, false);

    // Destination: the owner's associated token account (created idempotently).
    let ata = spl_associated_token_account::get_associated_token_address(&owner, &mint);

    // 1. Fetch the owner's compressed token accounts for this mint.
    let options = GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(Some(mint));
    let response = client
        .indexer()
        .map_err(|e| anyhow!("indexer unavailable (set --photon-url): {e}"))?
        .get_compressed_token_accounts_by_owner(&owner, Some(options), None)
        .await
        .map_err(|e| anyhow!("indexer query failed: {e}"))?;
    let accounts = response.value.items;
    if accounts.is_empty() {
        bail!("no compressed token accounts found for owner {owner} / mint {mint}");
    }

    let total: u64 = accounts.iter().map(|a| a.token.amount).sum();
    let amount = match amount_ui {
        Some(ui) => ui_to_base(ui, decimals)?,
        None => total,
    };
    if amount == 0 {
        bail!("nothing to decompress");
    }
    if amount > total {
        bail!("requested {amount} but only {total} available (base units)");
    }

    // 2. Validity proof over the input account hashes.
    let hashes: Vec<[u8; 32]> = accounts.iter().map(|a| a.account.hash).collect();
    let proof_ctx = client
        .get_validity_proof(hashes, vec![], None)
        .await
        .map_err(|e| anyhow!("get_validity_proof failed: {e}"))?
        .value;

    // 3. Pack input tree/queue accounts; indices are relative to the packed region.
    let mut packed = PackedAccounts::default();
    let packed_trees = proof_ctx.pack_tree_infos(&mut packed);
    let state = packed_trees
        .state_trees
        .ok_or_else(|| anyhow!("proof returned no state trees"))?;
    if state.packed_tree_infos.len() != accounts.len() {
        bail!(
            "proof/account mismatch: {} proofs for {} accounts",
            state.packed_tree_infos.len(),
            accounts.len()
        );
    }

    // 4. Build one input per compressed account (all share owner + mint).
    let owner_index = packed.insert_or_get_config(owner, true, false);
    let mint_index = packed.insert_or_get(mint);
    let mut inputs = Vec::with_capacity(accounts.len());
    for (acct, ti) in accounts.iter().zip(state.packed_tree_infos.iter()) {
        inputs.push(MultiInputTokenDataWithContext {
            owner: owner_index,
            amount: acct.token.amount,
            has_delegate: false,
            delegate: 0,
            mint: mint_index,
            version: account_version,
            merkle_context: PackedMerkleContext {
                merkle_tree_pubkey_index: ti.merkle_tree_pubkey_index,
                queue_pubkey_index: ti.queue_pubkey_index,
                prove_by_index: ti.prove_by_index,
                leaf_index: ti.leaf_index,
            },
            root_index: ti.root_index,
        });
    }

    // 5. Pool + destination ATA indices, then set up the SPL decompress.
    let pool_index = packed.insert_or_get(pool.pubkey);
    let recipient_index = packed.insert_or_get(ata);

    let mut ctoken = CTokenAccount2::new(inputs)
        .map_err(|e| anyhow!("building compressed token account: {e:?}"))?;
    ctoken
        .decompress_spl(amount, recipient_index, pool_index, 0, pool.bump, decimals)
        .map_err(|e| anyhow!("decompress_spl setup failed: {e:?}"))?;

    // 6. Assemble the transfer2 instruction.
    let (packed_metas, _, _) = packed.to_account_metas();
    let meta_config = Transfer2AccountsMetaConfig::new(owner, packed_metas);
    let transfer2 = create_transfer2_instruction(Transfer2Inputs {
        token_accounts: vec![ctoken],
        validity_proof: proof_ctx.proof,
        transfer_config: Transfer2Config::default().filter_zero_amount_outputs(),
        meta_config,
        output_queue: state.output_tree_index,
        ..Default::default()
    })
    .map_err(|e| anyhow!("building transfer2 instruction: {e:?}"))?;

    // 7. Ensure the destination ATA exists, then send.
    let create_ata = spl_associated_token_account::instruction::create_associated_token_account_idempotent(
        &owner,
        &owner,
        &mint,
        &token_program,
    );
    let instructions: Vec<Instruction> = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_000_000),
        create_ata,
        transfer2,
    ];

    let signature = client
        .create_and_send_transaction(&instructions, &owner, &[payer])
        .await
        .map_err(|e| anyhow!("decompress transaction failed: {e}"))?;

    println!("✅ decompressed {amount} base units of {mint}");
    println!("   into ATA: {ata}");
    println!("   tx: {signature}");
    println!("\nIt should now be visible in any standard wallet for {owner}.");
    Ok(())
}
