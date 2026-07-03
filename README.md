# zk-airdrop-cli

A Rust CLI that airdrops an SPL token to many wallets as **compressed tokens**
using [ZK Compression](https://www.zkcompression.com/) (Light Protocol).

Recipients receive compressed token accounts, so they pay **no per-recipient ATA
rent**, and each transaction fans out to many recipients at once via the
`batch_compress` primitive. This makes large airdrops dramatically cheaper than
distributing standard SPL tokens.

> Self-contained crate (own `[workspace]`), independent of the on-chain `senshi`
> program in the parent directory.

## How it works

1. **`create-mint`** — creates a standard SPL mint, registers its ZK-compression
   *token pool* (the SPL interface PDA that custodies the wrapped SPL supply),
   creates a source token account, and mints the full supply into it. All in one
   transaction.
2. **`airdrop`** — reads a CSV of recipients and, in batches, builds
   `batch_compress` instructions that pull tokens from the source account and
   write one compressed token account per recipient. Progress is checkpointed so
   a re-run resumes where it left off.
3. **`balance`** — queries the Photon indexer for an owner's compressed balance.

No validity proofs are needed for the airdrop itself: `batch_compress` only
*creates* new compressed accounts, and proofs are only required when *spending*
existing compressed state.

## Requirements

- Rust (stable) and a funded Solana keypair.
- An RPC endpoint that supports ZK Compression + a Photon indexer.
  [Helius](https://www.helius.dev/) serves both on one host. Plain
  `api.mainnet-beta.solana.com` will **not** work for `balance` / proofs.
- For local testing: the `light test-validator` from
  `@lightprotocol/zk-compression-cli` (bundles a validator + Photon + prover).

## Build

```bash
cargo build --release
```

## Usage

```bash
export KEYPAIR=~/.config/solana/id.json
export RPC_URL="https://devnet.helius-rpc.com?api-key=YOUR_KEY"
# PHOTON_URL defaults to RPC_URL (fine for Helius).

# 1. Create the mint + pool + funded source account (writes mint-info.json)
zk-airdrop create-mint --decimals 9 --supply 1000000

# 2. Airdrop — fixed amount per recipient
zk-airdrop airdrop \
  --mint <MINT_FROM_STEP_1> \
  --source-token-account <SOURCE_FROM_STEP_1> \
  --csv recipients.example.csv \
  --amount 100 --decimals 9 --batch-size 15

# 3. Check a recipient's compressed balance
zk-airdrop balance --mint <MINT> --owner <RECIPIENT>
```

### CSV format

`address` column required; `amount` column optional. A header row is
auto-detected. Amounts are **UI units** (whole tokens) and scaled by
`--decimals`.

```csv
address,amount
GdnSyH3YtwcxFvQrVVJMm1JhTS4QVX7MFsX56uJLUfiZ,100
9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM,250.5
```

If the CSV has no `amount` column, pass `--amount` for a fixed per-wallet amount.
`--amount` overrides per-row amounts when both are present.

### Resuming a failed run

Every confirmed batch is recorded in `--checkpoint`
(`airdrop-checkpoint.json` by default). Re-running the same command skips
already-confirmed batches, so partial failures are safe to retry.

## Tuning `--batch-size`

The cap is transaction *size* and *compute*, not logic. ~15 recipients/tx is a
safe default; raise it and watch for oversized-transaction or compute errors.
The CLI already raises the compute-unit limit to 1M per batch.

## Notes / caveats

- Pinned to the Light Protocol `0.23` crate line
  (`light-client`, `light-sdk`, `light-compressed-token-sdk` with the `v1`
  feature) so all shared crates resolve to a single version.
- The batch writes to a state tree from `get_random_state_tree_info()` (v2 trees
  by default). **Validate the full flow against `light test-validator` before
  running on mainnet** — tree/queue selection is the one piece that depends on
  the target cluster's deployment.
- This has been built and unit-tested; the on-chain path should be exercised on
  localnet/devnet first.
```
