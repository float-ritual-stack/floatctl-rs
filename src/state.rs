use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::Source;

const STATE_VERSION: u8 = 1;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplitState {
    pub version: u8,
    #[serde(default)]
    pub runs: Vec<RunRecord>,
    #[serde(default)]
    pub seen: BTreeMap<String, SeenRecord>,
}

impl Default for SplitState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            runs: Vec::new(),
            seen: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunRecord {
    pub run_id: String,
    pub input_fingerprint: String,
    pub processed: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SeenRecord {
    pub created_at: String,
    pub source: String,
    pub hash: String,
}

pub struct StateHandle {
    pub path: PathBuf,
    pub state: SplitState,
    lock_path: PathBuf,
    lock_file: Option<File>,
}

impl StateHandle {
    pub fn load(state_dir: &Path) -> Result<Self> {
        fs::create_dir_all(state_dir)
            .with_context(|| format!("failed to create state directory {}", state_dir.display()))?;

        let path = state_dir.join("conv_split.json");
        let lock_path = state_dir.join("conv_split.lock");
        let state = if path.exists() {
            let data = fs::read_to_string(&path)
                .with_context(|| format!("failed to read state file {}", path.display()))?;
            serde_json::from_str::<SplitState>(&data)
                .with_context(|| format!("failed to parse state file {}", path.display()))?
        } else {
            SplitState::default()
        };

        Ok(Self {
            path,
            state,
            lock_path,
            lock_file: None,
        })
    }

    pub fn acquire_lock(&mut self) -> Result<()> {
        if self.lock_file.is_some() {
            return Ok(());
        }
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.lock_path)
            .with_context(|| {
                format!("failed to acquire state lock {}", self.lock_path.display())
            })?;
        self.lock_file = Some(file);
        Ok(())
    }

    pub fn record_run(&mut self, run: RunRecord) {
        self.state.runs.push(run);
    }

    pub fn update_seen(
        &mut self,
        conversation_id: &str,
        created_at: &str,
        source: Source,
        hash: &str,
    ) {
        self.state.seen.insert(
            conversation_id.to_string(),
            SeenRecord {
                created_at: created_at.to_string(),
                source: source.as_str().to_string(),
                hash: hash.to_string(),
            },
        );
    }

    pub fn save(&mut self) -> Result<()> {
        self.acquire_lock()?;

        let tmp_path = self.path.with_file_name("conv_split.json.tmp");
        let data = serde_json::to_vec_pretty(&self.state).context("failed to serialize state")?;
        {
            let mut file = File::create(&tmp_path).with_context(|| {
                format!("failed to write temp state file {}", tmp_path.display())
            })?;
            file.write_all(&data)?;
            file.sync_all()
                .with_context(|| format!("failed to sync state file {}", tmp_path.display()))?;
        }

        fs::rename(&tmp_path, &self.path)
            .with_context(|| format!("failed to replace state file {}", self.path.display()))?;

        if let Some(lock) = self.lock_file.take() {
            drop(lock);
            let _ = fs::remove_file(&self.lock_path);
        }

        Ok(())
    }
}

impl Drop for StateHandle {
    fn drop(&mut self) {
        if let Some(lock) = self.lock_file.take() {
            drop(lock);
            let _ = fs::remove_file(&self.lock_path);
        }
    }
}
