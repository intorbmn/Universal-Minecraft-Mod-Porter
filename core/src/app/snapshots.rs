//! Checkpoint store location and persistence, shared by the CLI and the UI.

use super::AppError;
use crate::checkpoint::CheckpointStore;
use std::path::{Path, PathBuf};

const STORE_FILE: &str = "checkpoints.json";

/// `<project>.ummp-snapshots` next to the project folder.
pub fn default_snapshots(project_root: &Path) -> PathBuf {
    let name = project_root
        .file_name()
        .map_or_else(|| "project".into(), |n| n.to_string_lossy().into_owned());
    project_root.with_file_name(format!("{name}.ummp-snapshots"))
}

pub fn load_store(dir: &Path) -> Result<CheckpointStore, AppError> {
    let file = dir.join(STORE_FILE);
    if file.exists() {
        serde_json::from_slice(&std::fs::read(&file)?).map_err(|e| {
            AppError::Input(format!("{} is not a checkpoint store: {e}", file.display()))
        })
    } else {
        Ok(CheckpointStore::new())
    }
}

pub fn save_store(dir: &Path, store: &CheckpointStore) -> Result<(), AppError> {
    std::fs::create_dir_all(dir)?;
    let body = serde_json::to_vec_pretty(store)
        .map_err(|e| AppError::Input(format!("cannot serialize checkpoints: {e}")))?;
    std::fs::write(dir.join(STORE_FILE), body)?;
    Ok(())
}
