use anyhow::{anyhow, Result};
use light_client::{
    indexer::{GetCompressedTokenAccountsByOwnerOrDelegateOptions, Indexer},
    rpc::{LightClient, Rpc},
};
use solana_sdk::pubkey::Pubkey;

/// Print the total compressed balance an owner holds for a given mint.
pub async fn run(client: &LightClient, mint: Pubkey, owner: Pubkey) -> Result<()> {
    let options = GetCompressedTokenAccountsByOwnerOrDelegateOptions::new(Some(mint));

    let indexer = client
        .indexer()
        .map_err(|e| anyhow!("indexer unavailable (set --photon-url): {e}"))?;

    let response = indexer
        .get_compressed_token_accounts_by_owner(&owner, Some(options), None)
        .await
        .map_err(|e| anyhow!("indexer query failed: {e}"))?;

    let accounts = response.value.items;
    let total: u128 = accounts.iter().map(|a| a.token.amount as u128).sum();

    println!("owner {owner}");
    println!("mint  {mint}");
    println!(
        "compressed balance: {total} base units across {} compressed account(s)",
        accounts.len()
    );
    Ok(())
}
