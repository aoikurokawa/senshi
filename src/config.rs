use std::path::Path;

use anyhow::{anyhow, Context, Result};
use light_client::rpc::{LightClient, LightClientConfig};
use solana_sdk::signer::keypair::{read_keypair_file, Keypair};

/// Expand a leading `~` to the user's home directory.
fn expand_tilde(path: &Path) -> std::path::PathBuf {
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = std::env::var_os("HOME") {
            return Path::new(&home).join(stripped);
        }
    }
    path.to_path_buf()
}

/// Build a `LightClient` and return it alongside a separately-owned copy of the
/// payer keypair.
///
/// We keep our own `payer` copy because sending a transaction borrows the client
/// mutably, which would conflict with borrowing `client.payer` for signing.
pub async fn build_client(
    rpc_url: &str,
    photon_url: Option<&str>,
    keypair_path: &Path,
) -> Result<(LightClient, Keypair)> {
    let keypair_path = expand_tilde(keypair_path);
    let payer = read_keypair_file(&keypair_path)
        .map_err(|e| anyhow!("failed to read keypair {}: {e}", keypair_path.display()))?;

    // If no dedicated Photon URL is given, reuse the RPC URL — Helius endpoints
    // serve both the Solana RPC and the Photon indexer on the same host.
    let photon = Some(photon_url.unwrap_or(rpc_url).to_string());
    let config = LightClientConfig::new(rpc_url.to_string(), photon);

    let mut client = LightClient::new_with_retry(config, None)
        .await
        .map_err(|e| anyhow!("failed to initialize LightClient: {e}"))
        .context("is the RPC endpoint ZK-Compression aware (e.g. Helius)?")?;

    client.payer = payer.insecure_clone();
    Ok((client, payer))
}
