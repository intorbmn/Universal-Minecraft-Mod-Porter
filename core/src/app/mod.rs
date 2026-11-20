//! Application services: every action the CLI and the desktop UI can take (ADR 0004).
//!
//! The UI crate only calls these functions; no migration logic lives outside `core`.

pub mod helpers;
pub mod snapshots;
pub mod views;

use crate::adapter::loader::templates;
use crate::adapter::Handling;
use crate::analyzer::AnalyzerEngine;
use crate::filesystem::{FileSystem, LocalFileSystem};
use crate::migration::{MigrationEngine, MigrationPlanner, MigrationRequest};
use crate::port::stage_source;
use crate::project::{DefaultProjectManager, ProjectManager};
use crate::scan::{scan_path, Severity};
use helpers::{check_input, loader_label, open_kb, InputKind};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use views::{
    AnalysisView, FileDiffView, FileKind, FindingView, ModView, MoveView, PlanView, RiskLevel,
    RiskView, SkippedView, TargetVersion, TargetsView,
};

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

/// Preview request: the same inputs as a port, without an output folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanRequest {
    /// Source folder or `.zip`.
    pub input: PathBuf,
    /// Source Minecraft version; `None` = detected from the descriptor.
    pub from: Option<String>,
    pub to: String,
    /// Target loader; `None` = the mod's own loader.
    pub loader: Option<String>,
    pub min_confidence: f64,
}

const fn risk_level(s: Severity) -> RiskLevel {
    match s {
        Severity::Low => RiskLevel::Low,
        Severity::Medium => RiskLevel::Medium,
        Severity::High => RiskLevel::High,
    }
}

fn input_err(e: impl fmt::Display) -> AppError {
    AppError::Input(e.to_string())
}

fn slash(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
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

impl AppServices {
    fn knowledge(&self) -> Result<crate::knowledge::KnowledgeBase, AppError> {
        open_kb(self.settings().kb_path, None)
    }

    /// Detects mods, loader, versions and technologies. Reads only.
    pub fn analyze(&self, input: &Path) -> Result<AnalysisView, AppError> {
        let kind = check_input(input, false)?;
        let staging = tempfile::tempdir()?;
        let source = stage_source(input, staging.path())?;
        let project = DefaultProjectManager.open(&source).map_err(input_err)?;
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
                    loader: loader_label(&m.metadata.loader).unwrap_or("unknown").into(),
                    minecraft: m.metadata.minecraft.to_string(),
                    java: m.detected_java.as_ref().map(|j| j.major),
                })
                .collect(),
            build_system: report
                .build_system
                .as_ref()
                .map(|b| format!("{:?}", b.kind)),
            languages: report.languages,
            technologies: report.technologies,
        })
    }

    /// Static malware-risk scan of a folder, `.zip` or `.jar`. Reads only.
    pub fn scan(&self, input: &Path) -> Result<RiskView, AppError> {
        check_input(input, true)?;
        let report = scan_path(input).map_err(input_err)?;
        Ok(RiskView {
            level: report.risk().map(risk_level),
            findings: report
                .findings
                .into_iter()
                .map(|f| FindingView {
                    rule: f.rule,
                    severity: risk_level(f.severity),
                    file: f.file,
                    evidence: f.evidence,
                })
                .collect(),
        })
    }

    /// Loaders and the Minecraft versions UMMP ships build templates for.
    pub fn targets(&self) -> TargetsView {
        let mut versions: Vec<TargetVersion> = Vec::new();
        for (loader, mc) in templates() {
            match versions.iter_mut().find(|v| v.minecraft == mc) {
                Some(v) => v.loaders.push(loader.into()),
                None => versions.push(TargetVersion {
                    minecraft: mc.into(),
                    loaders: vec![loader.into()],
                }),
            }
        }
        TargetsView {
            loaders: ["forge", "fabric", "neoforge", "quilt"]
                .map(String::from)
                .to_vec(),
            versions,
        }
    }

    /// Computes every change a port or in-place migration would make. Writes nothing.
    pub fn plan(&self, req: &PlanRequest) -> Result<PlanView, AppError> {
        check_input(&req.input, false)?;
        let staging = tempfile::tempdir()?;
        let source = stage_source(&req.input, staging.path())?;
        let project = DefaultProjectManager.open(&source).map_err(input_err)?;
        let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
        let first = AnalyzerEngine::default()
            .analyze(&project.root, &fs)
            .ok()
            .and_then(|r| r.mods.into_iter().next());
        let source_loader = first
            .as_ref()
            .and_then(|m| loader_label(&m.metadata.loader))
            .map(String::from);
        let from = req
            .from
            .clone()
            .or_else(|| first.as_ref().map(|m| m.metadata.minecraft.to_string()))
            .ok_or_else(|| {
                AppError::Input(
                    "cannot detect the Minecraft version: choose the source version".into(),
                )
            })?;
        let target_loader = req
            .loader
            .clone()
            .or_else(|| source_loader.clone())
            .ok_or_else(|| {
                AppError::Input("cannot detect the loader: choose a target loader".into())
            })?
            .to_lowercase();
        let plan = MigrationPlanner::plan(
            &self.knowledge()?,
            &MigrationRequest {
                source_minecraft: from.clone(),
                target_minecraft: req.to.clone(),
                target_loader: target_loader.clone(),
                min_confidence: req.min_confidence,
                source_loader: source_loader.clone(),
            },
        )?;
        let set = MigrationEngine::default().compute_edits(&plan, &project)?;
        let needs_look = |file: &str| {
            set.technologies
                .iter()
                .any(|t| t.handling != Handling::Auto && t.file.replace('\\', "/") == file)
        };
        let files = set
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
                    needs_attention: needs_look(&path),
                    path,
                }
            })
            .collect();
        Ok(PlanView {
            source_minecraft: from,
            target_minecraft: req.to.clone(),
            source_loader,
            target_loader,
            files,
            moves: set
                .moves
                .iter()
                .map(|m| MoveView {
                    from: slash(&m.from),
                    to: slash(&m.to),
                })
                .collect(),
            deletes: set.deletes.iter().map(|p| slash(p)).collect(),
            skipped: plan
                .skipped
                .iter()
                .map(|s| SkippedView {
                    description: format!("{} -> {}", s.mapping.from_api, s.mapping.to_api),
                    reason: s.reason.clone(),
                })
                .collect(),
            technologies: set.technologies,
            warnings: set.warnings,
            files_scanned: set.files_scanned,
        })
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
