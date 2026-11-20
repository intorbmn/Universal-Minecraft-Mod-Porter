//! Small helpers the CLI used to own; now shared with the UI.

use super::AppError;
use crate::analyzer::AnalyzerEngine;
use crate::build_script::required_java;
use crate::filesystem::{FileSystem, LocalFileSystem};
use crate::knowledge::KnowledgeBase;
use crate::metadata::Loader;
use crate::modpack::plan::Side;
use crate::modpack::PackFormat;
use crate::scan::Severity;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Opens the knowledge base: a SQLite file, a mappings JSON loaded into memory, or an
/// empty in-memory one (structural migration only).
pub fn open_kb(kb: Option<PathBuf>, mappings: Option<PathBuf>) -> Result<KnowledgeBase, AppError> {
    Ok(match (kb, mappings) {
        (Some(db), _) => KnowledgeBase::open(&db)?,
        (None, Some(file)) => {
            let mem = KnowledgeBase::open(Path::new(":memory:"))?;
            mem.import_mappings_from_file(&file)?;
            mem
        }
        (None, None) => KnowledgeBase::open(Path::new(":memory:"))?,
    })
}

/// What a build request must do about the scan result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskDecision {
    Allow,
    /// Not local: runs on a disposable runner; tell the person, then go on.
    AllowWithWarning,
    /// Medium risk on this machine: ask first (the CLI prompts, the UI shows a checkbox).
    Confirm,
    Refuse(String),
}

/// Local builds run mod code on this machine: High risk needs `accept`, Medium needs
/// `yes` or `accept`. GitHub builds run on a disposable runner and only warn.
pub fn risk_decision(risk: Option<Severity>, local: bool, accept: bool, yes: bool) -> RiskDecision {
    if !local {
        return if risk.is_some() {
            RiskDecision::AllowWithWarning
        } else {
            RiskDecision::Allow
        };
    }
    match risk {
        Some(Severity::High) if !accept => RiskDecision::Refuse(
            "refusing to build a High-risk mod on this machine. \
             Build on GitHub (drop --local), or review the findings and pass --i-accept-risk"
                .into(),
        ),
        Some(Severity::Medium) if !yes && !accept => RiskDecision::Confirm,
        _ => RiskDecision::Allow,
    }
}

/// A folder- and branch-safe mod name, and the Java version the mod needs, if known.
pub fn mod_identity(path: &Path) -> (String, Option<u32>) {
    let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
    let first = AnalyzerEngine::default()
        .analyze(path, &fs)
        .ok()
        .and_then(|r| r.mods.into_iter().next());
    let raw = first
        .as_ref()
        .map(|m| m.metadata.mod_id.clone())
        .filter(|id| !id.is_empty())
        .or_else(|| path.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "mod".into());
    let name: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(40)
        .collect();
    let java = first.and_then(|m| {
        m.detected_java
            .map(|j| j.major)
            .or_else(|| m.metadata.java.map(|j| j.major))
            .or_else(|| required_java(&m.metadata.minecraft.to_string()))
    });
    (if name.is_empty() { "mod".into() } else { name }, java)
}

/// The loader named in the project's descriptor, lowercase (`fabric`, `neoforge`, ...).
pub fn detected_loader(path: &Path) -> Option<String> {
    let fs: Arc<dyn FileSystem> = Arc::new(LocalFileSystem);
    let report = AnalyzerEngine::default().analyze(path, &fs).ok()?;
    match &report.mods.first()?.metadata.loader {
        Loader::Forge => Some("forge".into()),
        Loader::Fabric => Some("fabric".into()),
        Loader::NeoForge => Some("neoforge".into()),
        Loader::Quilt => Some("quilt".into()),
        _ => None,
    }
}

pub fn pack_format(s: &str) -> Result<PackFormat, AppError> {
    match s {
        "mrpack" => Ok(PackFormat::Mrpack),
        "curseforge" => Ok(PackFormat::CurseForge),
        "prism" => Ok(PackFormat::Prism),
        "folder" => Ok(PackFormat::Folder),
        other => Err(AppError::Input(format!(
            "unknown pack format {other:?} (mrpack, curseforge, prism, folder)"
        ))),
    }
}

pub fn side(s: &str) -> Result<Side, AppError> {
    match s {
        "client" => Ok(Side::Client),
        "server" => Ok(Side::Server),
        "both" => Ok(Side::Both),
        other => Err(AppError::Input(format!(
            "unknown side {other:?} (client, server, both)"
        ))),
    }
}
