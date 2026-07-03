use std::{path::Path, str::FromStr};

use anyhow::{anyhow, bail, Context, Result};
use solana_sdk::pubkey::Pubkey;

/// A single airdrop recipient with an amount already scaled to base units.
#[derive(Debug, Clone)]
pub struct Recipient {
    pub address: Pubkey,
    pub amount: u64,
}

/// Convert a UI amount (whole tokens) to base units given the mint decimals.
pub fn ui_to_base(ui: f64, decimals: u8) -> Result<u64> {
    if ui < 0.0 {
        bail!("amount must be non-negative, got {ui}");
    }
    let base = ui * 10f64.powi(decimals as i32);
    if !base.is_finite() || base >= (u64::MAX as f64) {
        bail!("amount {ui} overflows u64 at {decimals} decimals");
    }
    Ok(base.round() as u64)
}

/// Load recipients from a CSV file.
///
/// Accepted rows: `address` or `address,amount`. A header line naming the
/// columns (e.g. `address,amount`) is auto-detected and skipped. Amounts in the
/// file are UI units and are scaled by `decimals`. When `default_amount` is set
/// it overrides every row's amount.
pub fn load_recipients(
    path: &Path,
    default_amount: Option<f64>,
    decimals: u8,
) -> Result<Vec<Recipient>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("opening CSV {}", path.display()))?;

    let mut out = Vec::new();
    for (i, record) in rdr.records().enumerate() {
        let record = record.with_context(|| format!("reading CSV row {}", i + 1))?;
        let addr_str = match record.get(0) {
            Some(s) if !s.is_empty() => s,
            _ => continue, // skip blank lines
        };

        // Detect and skip a header row.
        let address = match Pubkey::from_str(addr_str) {
            Ok(pk) => pk,
            Err(_) => {
                if i == 0 {
                    continue; // header like "address,amount"
                }
                return Err(anyhow!("invalid pubkey on row {}: {addr_str}", i + 1));
            }
        };

        let amount = match default_amount {
            Some(a) => ui_to_base(a, decimals)?,
            None => {
                let raw = record.get(1).ok_or_else(|| {
                    anyhow!(
                        "row {} has no amount column and --amount was not set",
                        i + 1
                    )
                })?;
                let ui: f64 = raw
                    .parse()
                    .with_context(|| format!("invalid amount on row {}: {raw}", i + 1))?;
                ui_to_base(ui, decimals)?
            }
        };

        out.push(Recipient { address, amount });
    }

    if out.is_empty() {
        bail!("no recipients found in {}", path.display());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(name: &str, contents: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        path
    }

    #[test]
    fn ui_scaling() {
        assert_eq!(ui_to_base(1.0, 9).unwrap(), 1_000_000_000);
        assert_eq!(ui_to_base(0.5, 6).unwrap(), 500_000);
        assert_eq!(ui_to_base(0.0, 9).unwrap(), 0);
        assert!(ui_to_base(-1.0, 9).is_err());
    }

    #[test]
    fn csv_with_header_and_amounts() {
        let pk1 = "11111111111111111111111111111111";
        let pk2 = "So11111111111111111111111111111111111111112";
        let path = write_tmp(
            "zk_airdrop_test_a.csv",
            &format!("address,amount\n{pk1},1\n{pk2},2.5\n"),
        );
        let r = load_recipients(&path, None, 9).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].amount, 1_000_000_000);
        assert_eq!(r[1].amount, 2_500_000_000);
    }

    #[test]
    fn csv_address_only_with_fixed_amount() {
        let pk1 = "11111111111111111111111111111111";
        let path = write_tmp("zk_airdrop_test_b.csv", &format!("{pk1}\n{pk1}\n"));
        let r = load_recipients(&path, Some(10.0), 6).unwrap();
        assert_eq!(r.len(), 2);
        assert!(r.iter().all(|x| x.amount == 10_000_000));
    }
}
