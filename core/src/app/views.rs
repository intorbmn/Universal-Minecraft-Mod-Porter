//! Data the UI shows. Plain, serializable, and small: the UI never sees engine types.

use crate::adapter::TechFinding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModView {
    pub id: String,
    pub name: String,
    pub version: String,
    /// `forge`, `fabric`, `neoforge`, `quilt` or `unknown`.
    pub loader: String,
    pub minecraft: String,
    pub java: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisView {
    /// `folder` or `zip`.
    pub kind: String,
    pub mods: Vec<ModView>,
    pub build_system: Option<String>,
    pub languages: Vec<String>,
    pub technologies: Vec<TechFinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingView {
    pub rule: String,
    pub severity: RiskLevel,
    pub file: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskView {
    /// `None` = no indicators found (not proof of safety).
    pub level: Option<RiskLevel>,
    pub findings: Vec<FindingView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetVersion {
    pub minecraft: String,
    /// Loaders UMMP has a full build template for at this version.
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetsView {
    pub loaders: Vec<String>,
    pub versions: Vec<TargetVersion>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    Modified,
    Created,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileDiffView {
    /// Relative to the project root, `/` separators.
    pub path: String,
    pub kind: FileKind,
    /// Unified diff, ready for display.
    pub diff: String,
    pub replacements: usize,
    /// An adapter finding in this file needs a human look (assisted or manual).
    pub needs_attention: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveView {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkippedView {
    pub description: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanView {
    pub source_minecraft: String,
    pub target_minecraft: String,
    pub source_loader: Option<String>,
    pub target_loader: String,
    pub files: Vec<FileDiffView>,
    pub moves: Vec<MoveView>,
    pub deletes: Vec<String>,
    pub technologies: Vec<TechFinding>,
    pub skipped: Vec<SkippedView>,
    pub warnings: Vec<String>,
    pub files_scanned: usize,
}
