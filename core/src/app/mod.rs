//! Application services: every action the CLI and the desktop UI can take (ADR 0004).
//!
//! The UI crate only calls these functions; no migration logic lives outside `core`.

pub mod helpers;
pub mod snapshots;

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const SETTINGS_FILE: &str = "settings.json";
const LANGUAGES: &[&str] = &["vi", "en"];

/// A key or token. Never `Serialize`, never printed (ADR 0004, phase 11 guide §1.2).
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw value, for handing to a provider or `gh`. Do not log it.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

/// Errors the services report. The message is meant to be shown to a person.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Input(String),
    #[error("{0}")]
    Refused(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Port(#[from] crate::port::PortError),
    #[error(transparent)]
    Pack(#[from] crate::modpack::PackError),
    #[error(transparent)]
    Migration(#[from] crate::migration::MigrationError),
    #[error(transparent)]
    Db(#[from] crate::persistence::DbError),
    #[error(transparent)]
    Checkpoint(#[from] crate::checkpoint::CheckpointError),
    #[error("cancelled")]
    Cancelled,
    #[error("not found: {0}")]
    NotFound(String),
}

/// Plain-text settings. No secrets: keys are passed per call as [`Secret`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Holds `jobs.db` and `settings.json`. Set by the caller, never read from the file.
    pub data_dir: PathBuf,
    /// `None` = built-in mappings only (in-memory knowledge base).
    pub kb_path: Option<PathBuf>,
    /// `None` = `port::default_port_root()` / `modpack::run::default_pack_root()`.
    pub output_root: Option<PathBuf>,
    /// The `UMMP_BUILD_REPO` equivalent.
    pub sandbox_repo: Option<String>,
    /// `anthropic` or `openai`.
    pub ai_provider: Option<String>,
    /// `vi` or `en`.
    pub language: String,
}

impl Settings {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir,
            kb_path: None,
            output_root: None,
            sandbox_repo: None,
            ai_provider: None,
            language: "en".into(),
        }
    }
}

pub struct AppServices {
    settings: Mutex<Settings>,
}

impl AppServices {
    /// Creates the data folder if needed and loads saved settings over the defaults.
    pub fn open(settings: Settings) -> Result<Self, AppError> {
        std::fs::create_dir_all(&settings.data_dir)?;
        let loaded = read_settings(&settings.data_dir)?.unwrap_or(settings);
        Ok(Self {
            settings: Mutex::new(loaded),
        })
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().expect("settings lock").clone()
    }

    pub fn save_settings(&self, s: Settings) -> Result<(), AppError> {
        if !LANGUAGES.contains(&s.language.as_str()) {
            return Err(AppError::Input(format!(
                "unknown language {:?} (vi, en)",
                s.language
            )));
        }
        // The data folder is fixed for the life of the instance.
        let s = Settings {
            data_dir: self.settings().data_dir,
            ..s
        };
        write_settings(&s)?;
        *self.settings.lock().expect("settings lock") = s;
        Ok(())
    }
}

fn read_settings(dir: &Path) -> Result<Option<Settings>, AppError> {
    let file = dir.join(SETTINGS_FILE);
    if !file.is_file() {
        return Ok(None);
    }
    let mut s: Settings = serde_json::from_slice(&std::fs::read(&file)?)
        .map_err(|e| AppError::Input(format!("{} is not valid settings: {e}", file.display())))?;
    s.data_dir = dir.to_path_buf();
    Ok(Some(s))
}

/// Writes through a temp file so a crash never leaves half a settings file.
fn write_settings(s: &Settings) -> Result<(), AppError> {
    let file = s.data_dir.join(SETTINGS_FILE);
    let tmp = s.data_dir.join(format!("{SETTINGS_FILE}.tmp"));
    let body = serde_json::to_vec_pretty(s)
        .map_err(|e| AppError::Input(format!("cannot serialize settings: {e}")))?;
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, &file)?;
    Ok(())
}
