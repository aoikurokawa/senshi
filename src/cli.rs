use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// A ZK Compression airdrop CLI for compressed SPL tokens on Solana.
///
/// Uses Light Protocol's `batch_compress` primitive to distribute an SPL token
/// to many recipients as *compressed* token accounts — recipients pay no ATA
/// rent, and each transaction fans out to many recipients at once.
#[derive(Parser, Debug)]
#[command(name = "zk-airdrop", version, about)]
pub struct Cli {
    /// Solana RPC endpoint. Must support ZK Compression (e.g. a Helius RPC).
    #[arg(long, env = "RPC_URL", global = true,
          default_value = "https://api.devnet.solana.com")]
    pub rpc_url: String,

    /// Photon indexer URL. Include an API key in the URL if required, e.g.
    /// `https://devnet.helius-rpc.com?api-key=YOUR_KEY`. Defaults to `rpc_url`.
    #[arg(long, env = "PHOTON_URL", global = true)]
    pub photon_url: Option<String>,

    /// Fee-payer / mint-authority keypair.
    #[arg(long, env = "KEYPAIR", global = true,
          default_value = "~/.config/solana/id.json")]
    pub keypair: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a fresh SPL mint, register its ZK-compression token pool, and mint
    /// the total supply into a source token account ready to be airdropped.
    CreateMint {
        /// Number of decimals for the new mint.
        #[arg(long, default_value_t = 9)]
        decimals: u8,

        /// Total supply to mint into the source account, in UI units
        /// (e.g. `1000000` = one million whole tokens).
        #[arg(long)]
        supply: f64,

        /// Where to write the resulting mint info JSON.
        #[arg(long, default_value = "mint-info.json")]
        out: PathBuf,
    },

    /// Airdrop compressed tokens to every recipient in a CSV file.
    Airdrop {
        /// The SPL mint to distribute.
        #[arg(long)]
        mint: String,

        /// Source SPL token account holding the supply (owned by the keypair).
        #[arg(long)]
        source_token_account: String,

        /// CSV of recipients. Columns: `address[,amount]` (header optional).
        #[arg(long)]
        csv: PathBuf,

        /// Fixed amount per recipient in UI units. Used when the CSV has no
        /// amount column; overrides per-row amounts when set.
        #[arg(long)]
        amount: Option<f64>,

        /// Mint decimals, used to convert UI amounts to base units.
        #[arg(long, default_value_t = 9)]
        decimals: u8,

        /// Recipients packed into each transaction.
        #[arg(long, default_value_t = 15)]
        batch_size: usize,

        /// Checkpoint file for idempotent resume.
        #[arg(long, default_value = "airdrop-checkpoint.json")]
        checkpoint: PathBuf,
    },

    /// Show the compressed token balance of an owner for a given mint.
    Balance {
        #[arg(long)]
        mint: String,
        #[arg(long)]
        owner: String,
    },
}
