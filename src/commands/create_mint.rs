use std::path::Path;

use anyhow::{anyhow, Result};
use light_client::rpc::{LightClient, Rpc};
use light_compressed_token_sdk::spl_interface::{derive_spl_interface_pda, CreateSplInterfacePda};
use serde::Serialize;
use solana_sdk::{
    instruction::Instruction,
    signer::{keypair::Keypair, Signer},
};
// `solana_sdk::system_instruction` is deprecated in favor of a separate crate,
// but re-exporting it here keeps the dependency surface small.
#[allow(deprecated)]
use solana_sdk::system_instruction;

use super::{SPL_MINT_LEN, SPL_TOKEN_ACCOUNT_LEN};
use crate::recipients::ui_to_base;

#[derive(Debug, Serialize)]
pub struct MintInfo {
    pub mint: String,
    pub source_token_account: String,
    pub token_pool: String,
    pub authority: String,
    pub decimals: u8,
    pub supply_ui: f64,
}

/// Create a mint, register its ZK-compression token pool, and mint the full
/// supply into a fresh source token account — all in one transaction.
pub async fn run(
    client: &mut LightClient,
    payer: &Keypair,
    decimals: u8,
    supply_ui: f64,
    out: &Path,
) -> Result<()> {
    let authority = payer.pubkey();
    let token_program = spl_token::id();

    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let source_kp = Keypair::new();
    let source = source_kp.pubkey();

    let mint_rent = client
        .get_minimum_balance_for_rent_exemption(SPL_MINT_LEN)
        .await
        .map_err(|e| anyhow!("rent lookup failed: {e}"))?;
    let acct_rent = client
        .get_minimum_balance_for_rent_exemption(SPL_TOKEN_ACCOUNT_LEN)
        .await
        .map_err(|e| anyhow!("rent lookup failed: {e}"))?;

    let supply = ui_to_base(supply_ui, decimals)?;

    let instructions: Vec<Instruction> = vec![
        // 1. Allocate + assign the mint account.
        system_instruction::create_account(
            &authority,
            &mint,
            mint_rent,
            SPL_MINT_LEN as u64,
            &token_program,
        ),
        // 2. Initialize the SPL mint (no freeze authority).
        spl_token::instruction::initialize_mint2(
            &token_program,
            &mint,
            &authority,
            None,
            decimals,
        )?,
        // 3. Register the ZK-compression token pool (SPL interface PDA) for the mint.
        CreateSplInterfacePda::new(authority, mint, token_program, false).instruction(),
        // 4. Allocate + assign the source token account.
        system_instruction::create_account(
            &authority,
            &source,
            acct_rent,
            SPL_TOKEN_ACCOUNT_LEN as u64,
            &token_program,
        ),
        // 5. Initialize the source token account owned by the authority.
        spl_token::instruction::initialize_account3(&token_program, &source, &mint, &authority)?,
        // 6. Mint the whole supply into the source account.
        spl_token::instruction::mint_to(
            &token_program,
            &mint,
            &source,
            &authority,
            &[],
            supply,
        )?,
    ];

    let signature = client
        .create_and_send_transaction(&instructions, &authority, &[payer, &mint_kp, &source_kp])
        .await
        .map_err(|e| anyhow!("create-mint transaction failed: {e}"))?;

    let pool = derive_spl_interface_pda(&mint, 0, false).pubkey;
    let info = MintInfo {
        mint: mint.to_string(),
        source_token_account: source.to_string(),
        token_pool: pool.to_string(),
        authority: authority.to_string(),
        decimals,
        supply_ui,
    };
    std::fs::write(out, serde_json::to_string_pretty(&info)?)?;

    println!("✅ mint created  (tx {signature})");
    println!("   mint:                 {}", info.mint);
    println!("   source token account: {}", info.source_token_account);
    println!("   token pool:           {}", info.token_pool);
    println!("   wrote {}", out.display());
    println!("\nNext: zk-airdrop airdrop --mint {} \\", info.mint);
    println!(
        "        --source-token-account {} --csv recipients.csv --decimals {}",
        info.source_token_account, decimals
    );
    Ok(())
}
