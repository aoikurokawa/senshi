mod checkpoint;
mod cli;
mod commands;
mod config;
mod recipients;

use std::str::FromStr;

use anyhow::{Context, Result};
use clap::Parser;
use solana_sdk::pubkey::Pubkey;

use crate::cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let (mut client, payer) =
        config::build_client(&args.rpc_url, args.photon_url.as_deref(), &args.keypair).await?;

    match args.command {
        Command::CreateMint {
            decimals,
            supply,
            out,
        } => {
            commands::create_mint::run(&mut client, &payer, decimals, supply, &out).await?;
        }
        Command::Airdrop {
            mint,
            source_token_account,
            csv,
            amount,
            decimals,
            batch_size,
            checkpoint,
        } => {
            let mint = Pubkey::from_str(&mint).context("invalid --mint")?;
            let source = Pubkey::from_str(&source_token_account)
                .context("invalid --source-token-account")?;
            let list = recipients::load_recipients(&csv, amount, decimals)?;
            println!("loaded {} recipients from {}", list.len(), csv.display());
            commands::airdrop::run(
                &mut client,
                &payer,
                mint,
                source,
                list,
                batch_size,
                checkpoint,
            )
            .await?;
        }
        Command::Balance { mint, owner } => {
            let mint = Pubkey::from_str(&mint).context("invalid --mint")?;
            let owner = Pubkey::from_str(&owner).context("invalid --owner")?;
            commands::balance::run(&client, mint, owner).await?;
        }
        Command::Decompress {
            mint,
            amount,
            decimals,
            account_version,
        } => {
            let mint = Pubkey::from_str(&mint).context("invalid --mint")?;
            commands::decompress::run(&mut client, &payer, mint, amount, decimals, account_version)
                .await?;
        }
    }

    Ok(())
}
