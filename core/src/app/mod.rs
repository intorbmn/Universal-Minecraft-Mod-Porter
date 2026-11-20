//! Application services: every action the desktop UI (and the CLI) can take, as plain
//! Rust functions. The UI crate only forwards to these (ADR 0004); it holds no logic.

pub mod helpers;
pub mod snapshots;
pub mod views;

use crate::adapter::{loader, Handling};
use crate::analyzer::AnalyzerEngine;
use crate::checkpoint::CheckpointError;
use crate::filesystem::{FileSystem, LocalFileSystem};
use crate::migration::{MigrationEngine, MigrationError, MigrationPlanner, MigrationRequest};
use crate::modpack::PackError;
use crate::persistence::DbError;
use crate::port::{stage_source, PortError};
use crate::project::{DefaultProjectManager, ProjectManager};
use crate::scan::{scan_path, Severity};
use helpers::{check_input, loader_label, open_kb, InputKind};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use views::{
    AnalysisView, FileDiffView, FileKind, FindingView, ModView, MoveView, PlanView, RiskLevel,
    RiskView, SkippedView, TargetVersion, TargetsView,
};

/// A key or token held in memory only. Not `Serialize`; prints as `***`.
#[derive(Clone)]
pub struct Secret(String);

impl Secret {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw value. Call this only where it is handed to a provider or backend.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("***")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The person's input is wrong; the message says what to choose instead.
    #[error("{0}")]
    Input(String),
    /// A safety rule stopped the action.
    #[error("{0}")]
    Refused(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Port(#[from] PortError),
    #[error(transparent)]
    Pack(#[from] PackError),
    #[error(transparent)]
    Migration(#[from] MigrationError),
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Checkpoint(#[from] CheckpointError),
    #[error("cancelled")]
    Cancelled,
    #[error("not found: {0}")]
    NotFound(String),
}

/// Stored in `settings.json`. Holds no keys or tokens: those are passed per call as [`Secret`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// Holds `jobs.db` and `settings.json`. Always the folder given to [`AppServices::open`].
    pub data_dir: PathBuf,
    /// `None` = built-in mappings only (in-memory knowledge base).
    pub kb_path: Option<PathBuf>,
    /// `None` = the default port and pack output roots.
    pub output_root: Option<PathBuf>,
    pub sandbox_repo: Option<String>,
    /// `"anthropic"` or `"openai"`.
    pub ai_provider: Option<String>,
    /// `"vi"` or `"en"`.
    pub language: String,
}

impl Settings {
    #[must_use]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanRequest {
    pub input: PathBuf,
    /// `None` = detect from the descriptor.
    pub from: Option<String>,
    pub to: String,
    /// `None` = keep the current loader.
    pub loader: Option<String>,
    pub min_confidence: f64,
}

const SETTINGS_FILE: &str = "settings.json";

pub struct AppServices {
    settings: Mutex<Settings>,
}

impl AppServices {
    /// Opens the services on `settings.data_dir`, creating it, and applies any saved
    /// `settings.json` over the defaults.
    ///
    /// # Errors
    /// Returns an error when the data folder cannot be created or `settings.json` is invalid.
    pub fn open(settings: Settings) -> Result<Self, AppError> {
        std::fs::create_dir_all(&settings.data_dir)?;
        let mut current = read_settings(&settings.data_dir)?.unwrap_or_else(|| settings.clone());
        current.data_dir = settings.data_dir;
        Ok(Self {
            settings: Mutex::new(current),
        })
    }

    #[must_use]
    pub fn settings(&self) -> Settings {
        self.settings.lock().expect("settings lock").clone()
    }

    /// Validates and saves the settings (`data_dir` never changes).
    ///
    /// # Errors
    /// Returns [`AppError::Input`] for an unknown language, or an I/O error.
    pub fn save_settings(&self, mut s: Settings) -> Result<(), AppError> {
        if !matches!(s.language.as_str(), "vi" | "en") {
            return Err(AppError::Input(format!(
                "unknown language {:?} (vi, en)",
                s.language
            )));
        }
        let mut guard = self.settings.lock().expect("settings lock");
        s.data_dir.clone_from(&guard.data_dir);
        write_settings(&s)?;
        *guard = s;
        drop(guard);
        Ok(())
    }

    /// What UMMP finds in a mod source folder or `.zip`. Writes nothing.
    ///
    /// # Errors
    /// Returns [`AppError::Input`] when the path is wrong or holds no mod.
    pub fn analyze(&self, input: &Path) -> Result<AnalysisView, AppError> {
        let kind = check_input(input, false)?;
        let staging = tempfile::tempdir()?;
        let root = stage_source(input, staging.path())?;
        let project = DefaultProjectManager.open(&root).map_err(input_err)?;
        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let report = AnalyzerEngine::default()
            .analyze(&project.root, &fs)
            .map_err(input_err)?;
        if report.mods.is_empty() {
            return Err(AppError::Input(format!(
                "no mod found in {}. Choose the mod's source folder (it contains the descriptor, e.g. fabric.mod.json).",
                input.display()
            )));
        }
        Ok(AnalysisView {
            kind: if kind == InputKind::Zip {
                "zip"
            } else {
                "folder"
            }
            .into(),
            mods: report
                .mods
                .iter()
                .map(|m| ModView {
                    id: m.metadata.mod_id.clone(),
                    name: m.metadata.name.clone(),
                    version: m.metadata.version.clone(),
                    loader: loader_label(&m.metadata.loader)
                        .map_or_else(|| format!("{:?}", m.metadata.loader), str::to_owned),
                    minecraft: m.metadata.minecraft.to_string(),
                    java: m.detected_java.as_ref().map(|j| j.major),
                })
                .collect(),
            build_system: report.build_system.map(|b| format!("{:?}", b.kind)),
            languages: report.languages,
            technologies: report.technologies,
        })
    }

    /// Malware-risk indicators in a folder, `.zip` or `.jar`. Never runs anything.
    ///
    /// # Errors
    /// Returns [`AppError::Input`] when the path is wrong or unreadable.
    pub fn scan(&self, input: &Path) -> Result<RiskView, AppError> {
        check_input(input, true)?;
        let report = scan_path(input).map_err(input_err)?;
        Ok(RiskView {
            level: report.risk().map(risk_level),
            findings: report
                .findings
                .iter()
                .map(|f| FindingView {
                    rule: f.rule.clone(),
                    severity: risk_level(f.severity),
                    file: f.file.clone(),
                    evidence: f.evidence.clone(),
                })
                .collect(),
        })
    }

    /// Every loader the converter knows, and the Minecraft versions UMMP ships build
    /// templates for (ports to other versions stay possible through `plan`).
    #[must_use]
    pub fn targets(&self) -> TargetsView {
        let mut versions: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut order: Vec<String> = Vec::new();
        for (l, mc) in loader::templates() {
            if !versions.contains_key(mc) {
                order.push(mc.to_owned());
            }
            let entry = versions.entry(mc.to_owned()).or_default();
            if !entry.iter().any(|x| x == l) {
                entry.push(l.to_owned());
            }
        }
        TargetsView {
            loaders: ["forge", "fabric", "neoforge", "quilt"]
                .map(str::to_owned)
                .to_vec(),
            versions: order
                .into_iter()
                .map(|mc| TargetVersion {
                    loaders: versions.remove(&mc).unwrap_or_default(),
                    minecraft: mc,
                })
                .collect(),
        }
    }

    /// The edits a migration would make, as diffs. Never writes.
    ///
    /// # Errors
    /// Returns [`AppError::Input`] for bad input or an undetectable version or loader, and
    /// [`AppError::Migration`] when the planner refuses (for example a downgrade).
    pub fn plan(&self, req: &PlanRequest) -> Result<PlanView, AppError> {
        check_input(&req.input, false)?;
        let staging = tempfile::tempdir()?;
        let root = stage_source(&req.input, staging.path())?;
        let project = DefaultProjectManager.open(&root).map_err(input_err)?;
        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let report = AnalyzerEngine::default()
            .analyze(&project.root, &fs)
            .map_err(input_err)?;
        let first = report.mods.first();
        let source_loader = first.and_then(|m| loader_label(&m.metadata.loader).map(str::to_owned));
        let from = match (&req.from, first) {
            (Some(f), _) => f.clone(),
            (None, Some(m)) => m.metadata.minecraft.to_string(),
            (None, None) => {
                return Err(AppError::Input(
                    "cannot detect the Minecraft version: choose the source version".into(),
                ))
            }
        };
        let target_loader = match (&req.loader, &source_loader) {
            (Some(l), _) => l.to_ascii_lowercase(),
            (None, Some(l)) => l.clone(),
            (None, None) => {
                return Err(AppError::Input(
                    "cannot detect the loader: choose a target loader".into(),
                ))
            }
        };
        let settings = self.settings();
        let kb = open_kb(settings.kb_path, None)?;
        let request = MigrationRequest {
            source_minecraft: from.clone(),
            target_minecraft: req.to.clone(),
            target_loader: target_loader.clone(),
            min_confidence: req.min_confidence,
            source_loader: source_loader.clone(),
        };
        let plan = MigrationPlanner::plan(&kb, &request)?;
        let set = MigrationEngine::default().compute_edits(&plan, &project)?;
        let attention: Vec<String> = set
            .technologies
            .iter()
            .filter(|t| t.handling != Handling::Auto)
            .map(|t| t.file.replace('\\', "/"))
            .collect();
        Ok(PlanView {
            source_minecraft: from,
            target_minecraft: req.to.clone(),
            source_loader,
            target_loader,
            files: set
                .edits
                .iter()
                .map(|e| {
                    let path = slash(&e.path);
                    FileDiffView {
                        kind: if e.original.is_empty() {
                            FileKind::Created
                        } else {
                            FileKind::Modified
                        },
                        diff: e.unified_diff(),
                        replacements: e.replacements.len(),
                        needs_attention: attention.iter().any(|a| *a == path),
                        path,
                    }
                })
                .collect(),
            moves: set
                .moves
                .iter()
                .map(|m| MoveView {
                    from: slash(&m.from),
                    to: slash(&m.to),
                })
                .collect(),
            deletes: set.deletes.iter().map(|p| slash(p)).collect(),
            technologies: set.technologies,
            skipped: plan
                .skipped
                .iter()
                .map(|s| SkippedView {
                    description: format!("{} -> {}", s.mapping.from_api, s.mapping.to_api),
                    reason: s.reason.clone(),
                })
                .collect(),
            warnings: set.warnings,
            files_scanned: set.files_scanned,
        })
    }
}

fn read_settings(data_dir: &Path) -> Result<Option<Settings>, AppError> {
    let file = data_dir.join(SETTINGS_FILE);
    if !file.exists() {
        return Ok(None);
    }
    serde_json::from_slice(&std::fs::read(&file)?)
        .map(Some)
        .map_err(|e| AppError::Input(format!("{} is not valid: {e}", file.display())))
}

fn write_settings(s: &Settings) -> Result<(), AppError> {
    let json = serde_json::to_vec_pretty(s)
        .map_err(|e| AppError::Input(format!("cannot write settings: {e}")))?;
    let tmp = s.data_dir.join("settings.json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, s.data_dir.join(SETTINGS_FILE))?;
    Ok(())
}

/// Wraps an engine error that has no variant of its own as a message for the person.
fn input_err(e: impl std::fmt::Display) -> AppError {
    AppError::Input(e.to_string())
}

fn slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

const fn risk_level(s: Severity) -> RiskLevel {
    match s {
        Severity::Low => RiskLevel::Low,
        Severity::Medium => RiskLevel::Medium,
        Severity::High => RiskLevel::High,
    }
}
