use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Persisted airdrop progress so a re-run skips already-confirmed batches.
///
/// Keyed by batch index; the value is the confirmed transaction signature.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Checkpoint {
    pub completed: BTreeMap<usize, String>,
    #[serde(skip)]
    path: PathBuf,
}

impl Checkpoint {
    /// Load an existing checkpoint or start a fresh one bound to `path`.
    pub fn load(path: PathBuf) -> Result<Self> {
        if path.exists() {
            let data = std::fs::read_to_string(&path)
                .with_context(|| format!("reading checkpoint {}", path.display()))?;
            let mut cp: Checkpoint =
                serde_json::from_str(&data).context("parsing checkpoint JSON")?;
            cp.path = path;
            Ok(cp)
        } else {
            Ok(Checkpoint {
                completed: BTreeMap::new(),
                path,
            })
        }
    }

    pub fn is_done(&self, batch: usize) -> bool {
        self.completed.contains_key(&batch)
    }

    /// Record a confirmed batch and flush to disk immediately.
    pub fn mark_done(&mut self, batch: usize, signature: String) -> Result<()> {
        self.completed.insert(batch, signature);
        self.save()
    }

    fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(self).context("serializing checkpoint")?;
        std::fs::write(&self.path, data)
            .with_context(|| format!("writing checkpoint {}", self.path.display()))?;
        Ok(())
    }
}
