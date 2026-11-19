//! Application: migration engine (Phase 6 MVP).
//!
//! Pipeline slice implemented here: **Plan → Transform → Review (diff/report)
//! → Apply (with checkpoint) → Rollback**.
//!
//! - The planner selects API mappings from the knowledge base for the target
//!   loader whose version lies in `(source, target]` and groups them into
//!   ordered *stages*, one per Minecraft version. Stages run in order so a
//!   chain `A→B` (1.20) then `B→C` (1.21) ends at `C`.
//! - Source transformers implement [`SourceTransformer`]. The MVP ships
//!   [`TokenRenameTransformer`] for Java/Kotlin (ADR 0007).
//! - Edits are applied only after a checkpoint is taken; any write failure
//!   restores the checkpoint (ADR 0005, ADR 0006).

use crate::checkpoint::{collect_files, CheckpointError, CheckpointStore};
use crate::filesystem::{resolve_within, FsError};
use crate::knowledge::{BreakingChange, KnowledgeBase, Mapping};
use crate::persistence::DbError;
use crate::project::Project;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// Files larger than this are reported and skipped, never transformed.
pub const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("knowledge base: {0}")]
    Db(#[from] DbError),
    #[error("fs: {0}")]
    Fs(#[from] FsError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("checkpoint: {0}")]
    Checkpoint(#[from] CheckpointError),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("apply failed on {path} ({cause}); rolled back: {rolled_back}")]
    ApplyFailed {
        path: PathBuf,
        cause: String,
        rolled_back: bool,
    },
}

// ---------------------------------------------------------------------------
// Request / plan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationRequest {
    pub source_minecraft: String,
    pub target_minecraft: String,
    pub target_loader: String,
    /// Mappings below this confidence are reported, not applied.
    pub min_confidence: f64,
}

/// One API rename taken from a knowledge-base [`Mapping`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenameRule {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub confidence: f64,
    pub version: String,
}

impl RenameRule {
    /// `true` for dotted names such as `net.minecraft.block.Block`.
    pub fn is_qualified(&self) -> bool {
        self.from.contains('.')
    }
}

/// All rules introduced by one Minecraft version, applied together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub version: String,
    pub rules: Vec<RenameRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedMapping {
    pub mapping: Mapping,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    pub request: MigrationRequest,
    pub stages: Vec<Stage>,
    pub skipped: Vec<SkippedMapping>,
    pub breaking_changes: Vec<BreakingChange>,
    pub recipes: Vec<String>,
}

impl MigrationPlan {
    pub fn rule_count(&self) -> usize {
        self.stages.iter().map(|s| s.rules.len()).sum()
    }
}

fn parse_version(v: &str) -> Option<Vec<u32>> {
    v.split('.').map(|p| p.parse().ok()).collect()
}

/// Compare dotted numeric versions, padding with zeros (`1.21` == `1.21.0`).
pub fn compare_versions(a: &str, b: &str) -> Option<Ordering> {
    let (mut a, mut b) = (parse_version(a)?, parse_version(b)?);
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    Some(a.cmp(&b))
}

/// `true` when `v` is in `(source, target]`.
fn in_upgrade_range(v: &str, source: &str, target: &str) -> bool {
    matches!(compare_versions(v, source), Some(Ordering::Greater))
        && matches!(
            compare_versions(v, target),
            Some(Ordering::Less | Ordering::Equal)
        )
}

pub struct MigrationPlanner;

impl MigrationPlanner {
    pub fn plan(
        kb: &KnowledgeBase,
        request: &MigrationRequest,
    ) -> Result<MigrationPlan, MigrationError> {
        let (src, tgt) = (&request.source_minecraft, &request.target_minecraft);
        match compare_versions(src, tgt) {
            None => {
                return Err(MigrationError::InvalidRequest(format!(
                    "versions must be numeric (got '{src}' -> '{tgt}')"
                )))
            }
            Some(Ordering::Greater) => {
                return Err(MigrationError::InvalidRequest(format!(
                    "downgrades are not supported ('{src}' -> '{tgt}')"
                )))
            }
            _ => {}
        }

        let mut skipped = Vec::new();
        // version -> from -> rule
        let mut by_version: BTreeMap<Vec<u32>, (String, BTreeMap<String, RenameRule>)> =
            BTreeMap::new();
        let mut conflicts: Vec<(Vec<u32>, String)> = Vec::new();

        for m in kb.mappings_by_loader(&request.target_loader)? {
            if !in_upgrade_range(&m.version, src, tgt) {
                continue;
            }
            if m.confidence < request.min_confidence {
                skipped.push(SkippedMapping {
                    reason: format!(
                        "confidence {:.2} below threshold {:.2}",
                        m.confidence, request.min_confidence
                    ),
                    mapping: m,
                });
                continue;
            }
            if m.from_api == m.to_api || m.from_api.is_empty() || m.to_api.is_empty() {
                skipped.push(SkippedMapping {
                    reason: "no-op or empty mapping".into(),
                    mapping: m,
                });
                continue;
            }
            let key = parse_version(&m.version).unwrap_or_default();
            let entry = by_version
                .entry(key.clone())
                .or_insert_with(|| (m.version.clone(), BTreeMap::new()));
            match entry.1.get(&m.from_api) {
                Some(existing) if existing.to != m.to_api => {
                    conflicts.push((key, m.from_api.clone()));
                    skipped.push(SkippedMapping {
                        reason: format!(
                            "conflicts with '{}' -> '{}' for the same version",
                            existing.from, existing.to
                        ),
                        mapping: m,
                    });
                }
                Some(_) => {} // duplicate of an identical rule
                None => {
                    entry.1.insert(
                        m.from_api.clone(),
                        RenameRule {
                            from: m.from_api.clone(),
                            to: m.to_api.clone(),
                            kind: m.kind.clone(),
                            confidence: m.confidence,
                            version: m.version.clone(),
                        },
                    );
                }
            }
        }
        // A conflicting name is ambiguous: drop every rule for it in that version.
        for (key, from) in conflicts {
            if let Some((_, rules)) = by_version.get_mut(&key) {
                if let Some(rule) = rules.remove(&from) {
                    skipped.push(SkippedMapping {
                        reason: "ambiguous: several targets for the same name".into(),
                        mapping: Mapping {
                            version: rule.version,
                            loader: request.target_loader.clone(),
                            from_api: rule.from,
                            to_api: rule.to,
                            kind: rule.kind,
                            source: "planner".into(),
                            confidence: rule.confidence,
                        },
                    });
                }
            }
        }

        let stages = by_version
            .into_values()
            .filter(|(_, rules)| !rules.is_empty())
            .map(|(version, rules)| Stage {
                version,
                rules: rules.into_values().collect(),
            })
            .collect();

        // `search_breaking_changes("")` matches every row (LIKE '%%').
        let breaking_changes = kb
            .search_breaking_changes("")?
            .into_iter()
            .filter(|b| in_upgrade_range(&b.version, src, tgt))
            .collect();
        let recipes = kb
            .recipes_by_loader(&request.target_loader)?
            .into_iter()
            .filter(|r| in_upgrade_range(&r.version, src, tgt))
            .map(|r| r.recipe_id)
            .collect();

        Ok(MigrationPlan {
            request: request.clone(),
            stages,
            skipped,
            breaking_changes,
            recipes,
        })
    }
}

// ---------------------------------------------------------------------------
// Source transformation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub line: usize,
    pub from: String,
    pub to: String,
}

/// Where a candidate match sits, used to decide whether to rewrite it.
#[derive(Debug, Clone, Copy)]
pub struct MatchContext<'a> {
    pub rule: &'a RenameRule,
    /// The matched name follows a `.` (e.g. `foo.Block`, `a.b.Block`).
    pub preceded_by_dot: bool,
    /// The name is part of an `import` / `import static` statement.
    pub in_import: bool,
}

/// Rewrite policy for a token that matches a rule.
///
/// Without type information we cannot tell whether `x.Block` refers to the
/// class `Block` or to an unrelated member called `Block`, so a *simple-name*
/// rule only rewrites names that are not member accesses. Qualified rules and
/// import statements are unambiguous: they name a type by its full path.
pub fn should_rewrite(ctx: &MatchContext<'_>) -> bool {
    if ctx.rule.is_qualified() || ctx.in_import {
        return true;
    }
    !ctx.preceded_by_dot
}

pub trait SourceTransformer: Send + Sync {
    fn name(&self) -> &'static str;
    fn applies_to(&self, path: &Path) -> bool;
    /// Apply one stage of rules. Must not change anything else in `content`.
    fn transform(&self, content: &str, rules: &[RenameRule]) -> (String, Vec<Replacement>);
}

/// Token-aware renamer for Java and Kotlin sources (ADR 0007).
///
/// Matches whole identifiers or dotted paths only, never inside a longer
/// identifier, and never inside comments, string, char or text-block literals.
pub struct TokenRenameTransformer;

const fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$' || b >= 0x80
}

const fn is_ident_cont(b: u8) -> bool {
    is_ident_start(b) || b.is_ascii_digit()
}

fn find_seq(b: &[u8], from: usize, pat: &[u8]) -> Option<usize> {
    b.get(from..)?
        .windows(pat.len())
        .position(|w| w == pat)
        .map(|p| p + from)
}

/// Skip a `"…"` or `'…'` literal starting at `i`; stops at an unescaped
/// closing quote or at end of line (unterminated literal).
fn skip_quoted(b: &[u8], i: usize, quote: u8) -> usize {
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'\n' => return j,
            c if c == quote => return j + 1,
            _ => j += 1,
        }
    }
    b.len()
}

impl SourceTransformer for TokenRenameTransformer {
    fn name(&self) -> &'static str {
        "token-rename"
    }

    /// Java and Kotlin sources. Build scripts (`*.gradle.kts`) are excluded:
    /// they belong to the not-yet-implemented build migration step.
    fn applies_to(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("java" | "kt")
        )
    }

    fn transform(&self, src: &str, rules: &[RenameRule]) -> (String, Vec<Replacement>) {
        let bytes = src.as_bytes();
        let src_len = bytes.len();
        let rule_segs: Vec<(Vec<&str>, &RenameRule)> = rules
            .iter()
            .map(|r| (r.from.split('.').collect(), r))
            .collect();

        let mut out = String::with_capacity(src_len);
        let mut reps = Vec::new();
        let mut copied = 0; // bytes of `src` already copied to `out`
        let (mut line, mut line_pos) = (1, 0); // incremental line counter
        let mut prev_words: [Option<&str>; 2] = [None, None];
        let mut i = 0;

        while i < src_len {
            let c = bytes[i];
            if c == b'/' && bytes.get(i + 1) == Some(&b'/') {
                i = find_seq(bytes, i, b"\n").unwrap_or(src_len);
                prev_words = [None, None];
                continue;
            }
            if c == b'/' && bytes.get(i + 1) == Some(&b'*') {
                i = find_seq(bytes, i + 2, b"*/").map_or(src_len, |p| p + 2);
                continue;
            }
            if c == b'"' {
                i = if bytes[i..].starts_with(b"\"\"\"") {
                    find_seq(bytes, i + 3, b"\"\"\"").map_or(src_len, |p| p + 3)
                } else {
                    skip_quoted(bytes, i, b'"')
                };
                prev_words = [None, None];
                continue;
            }
            if c == b'\'' {
                i = skip_quoted(bytes, i, b'\'');
                prev_words = [None, None];
                continue;
            }
            if c.is_ascii_digit() {
                // numeric literal such as 0xFF or 1.5f: never an identifier
                while i < src_len && is_ident_cont(bytes[i]) {
                    i += 1;
                }
                continue;
            }
            if !is_ident_start(c) {
                if !c.is_ascii_whitespace() {
                    prev_words = [None, None];
                }
                i += 1;
                continue;
            }

            // Dotted path: ident ( '.' ident )*
            let mut segs: Vec<(usize, usize)> = Vec::new();
            let mut j = i;
            loop {
                let s = j;
                while j < src_len && is_ident_cont(bytes[j]) {
                    j += 1;
                }
                segs.push((s, j));
                if j + 1 < src_len && bytes[j] == b'.' && is_ident_start(bytes[j + 1]) {
                    j += 1;
                } else {
                    break;
                }
            }

            let in_import = prev_words[0] == Some("import")
                || (prev_words[0] == Some("static") && prev_words[1] == Some("import"));
            let lead_dot = i > 0 && bytes[i - 1] == b'.';

            let mut k = 0;
            while k < segs.len() {
                let mut best: Option<(usize, &RenameRule)> = None;
                for (rs, rule) in &rule_segs {
                    let len = rs.len();
                    if k + len > segs.len() || best.is_some_and(|(bl, _)| bl >= len) {
                        continue;
                    }
                    let matches = rs
                        .iter()
                        .enumerate()
                        .all(|(t, seg)| &src[segs[k + t].0..segs[k + t].1] == *seg);
                    let ctx = MatchContext {
                        rule,
                        preceded_by_dot: k > 0 || lead_dot,
                        in_import,
                    };
                    if matches && should_rewrite(&ctx) {
                        best = Some((len, rule));
                    }
                }
                if let Some((len, rule)) = best {
                    let (start, end) = (segs[k].0, segs[k + len - 1].1);
                    line += src[line_pos..start].bytes().filter(|&x| x == b'\n').count();
                    line_pos = start;
                    out.push_str(&src[copied..start]);
                    out.push_str(&rule.to);
                    copied = end;
                    reps.push(Replacement {
                        line,
                        from: rule.from.clone(),
                        to: rule.to.clone(),
                    });
                    k += len;
                } else {
                    k += 1;
                }
            }

            prev_words = [Some(&src[i..j]), prev_words[0]];
            i = j;
        }
        out.push_str(&src[copied..]);
        (out, reps)
    }
}

// ---------------------------------------------------------------------------
// Edits, diff, report
// ---------------------------------------------------------------------------

/// A proposed change to one file. Both rule-based transforms and (later) AI
/// agents produce `FileEdit`s; only the engine applies them (ADR 0005).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdit {
    /// Path relative to the project root.
    pub path: PathBuf,
    pub original: String,
    pub updated: String,
    pub replacements: Vec<Replacement>,
}

impl FileEdit {
    pub fn unified_diff(&self) -> String {
        let name = self.path.to_string_lossy().replace('\\', "/");
        similar::TextDiff::from_lines(&self.original, &self.updated)
            .unified_diff()
            .context_radius(3)
            .header(&format!("a/{name}"), &format!("b/{name}"))
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChangeSummary {
    pub path: PathBuf,
    pub replacements: Vec<Replacement>,
    pub diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    pub request: MigrationRequest,
    pub dry_run: bool,
    pub checkpoint_id: Option<Uuid>,
    pub files_scanned: usize,
    pub changes: Vec<FileChangeSummary>,
    pub skipped_mappings: Vec<SkippedMapping>,
    pub breaking_changes: Vec<BreakingChange>,
    pub recipes: Vec<String>,
    pub warnings: Vec<String>,
}

impl MigrationReport {
    pub fn total_replacements(&self) -> usize {
        self.changes.iter().map(|c| c.replacements.len()).sum()
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn to_markdown(&self) -> String {
        let r = &self.request;
        let mut out = String::new();
        let _ = writeln!(out, "# Migration Report\n");
        let _ = writeln!(
            out,
            "- Target: Minecraft {} -> {} on `{}`",
            r.source_minecraft, r.target_minecraft, r.target_loader
        );
        let mode = if self.dry_run {
            "dry run (nothing written)"
        } else {
            "applied"
        };
        let _ = writeln!(out, "- Mode: {mode}");
        if let Some(id) = self.checkpoint_id {
            let _ = writeln!(out, "- Checkpoint: `{id}` (use it to roll back)");
        }
        let _ = writeln!(
            out,
            "- Files scanned: {}, files changed: {}, replacements: {}\n",
            self.files_scanned,
            self.changes.len(),
            self.total_replacements()
        );

        if !self.breaking_changes.is_empty() {
            let _ = writeln!(out, "## Breaking changes to review manually\n");
            for b in &self.breaking_changes {
                let _ = writeln!(out, "- **{}** [{}] {}", b.version, b.severity, b.change);
                if let Some(w) = &b.workaround {
                    let _ = writeln!(out, "  - Workaround: {w}");
                }
            }
            out.push('\n');
        }
        if !self.recipes.is_empty() {
            let _ = writeln!(out, "## Applicable recipes (not executed yet)\n");
            for id in &self.recipes {
                let _ = writeln!(out, "- `{id}`");
            }
            out.push('\n');
        }
        if !self.skipped_mappings.is_empty() {
            let _ = writeln!(out, "## Mappings not applied\n");
            for s in &self.skipped_mappings {
                let _ = writeln!(
                    out,
                    "- `{}` -> `{}` ({}): {}",
                    s.mapping.from_api, s.mapping.to_api, s.mapping.version, s.reason
                );
            }
            out.push('\n');
        }
        if !self.warnings.is_empty() {
            let _ = writeln!(out, "## Warnings\n");
            for w in &self.warnings {
                let _ = writeln!(out, "- {w}");
            }
            out.push('\n');
        }
        if !self.changes.is_empty() {
            let _ = writeln!(out, "## Diff\n");
            for c in &self.changes {
                let _ = writeln!(out, "```diff\n{}```\n", c.diff);
            }
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Engine
// ---------------------------------------------------------------------------

pub struct RunOptions<'a> {
    pub dry_run: bool,
    /// Where checkpoints are stored; must be outside the project.
    pub snapshot_root: &'a Path,
    pub session_id: Uuid,
}

pub struct MigrationEngine {
    transformers: Vec<Box<dyn SourceTransformer>>,
}

impl Default for MigrationEngine {
    fn default() -> Self {
        Self {
            transformers: vec![Box::new(TokenRenameTransformer)],
        }
    }
}

impl MigrationEngine {
    pub fn new(transformers: Vec<Box<dyn SourceTransformer>>) -> Self {
        Self { transformers }
    }

    /// Compute edits without writing anything.
    /// Returns `(edits, files_scanned, warnings)`.
    pub fn compute_edits(
        &self,
        plan: &MigrationPlan,
        project: &Project,
    ) -> Result<(Vec<FileEdit>, usize, Vec<String>), MigrationError> {
        let mut edits = Vec::new();
        let mut warnings = Vec::new();
        let mut scanned = 0;

        for rel in collect_files(&project.root)? {
            let Some(tf) = self.transformers.iter().find(|t| t.applies_to(&rel)) else {
                continue;
            };
            let abs = resolve_within(&project.root, &rel)?;
            if std::fs::metadata(&abs)?.len() > MAX_SOURCE_BYTES {
                warnings.push(format!(
                    "{}: larger than {MAX_SOURCE_BYTES} bytes, skipped",
                    rel.display()
                ));
                continue;
            }
            scanned += 1;
            let Ok(original) = String::from_utf8(project.fs.read(&abs)?) else {
                warnings.push(format!("{}: not valid UTF-8, skipped", rel.display()));
                continue;
            };

            let mut current = original.clone();
            let mut replacements = Vec::new();
            for stage in &plan.stages {
                let (next, reps) = tf.transform(&current, &stage.rules);
                current = next;
                replacements.extend(reps);
            }
            if current != original {
                edits.push(FileEdit {
                    path: rel,
                    original,
                    updated: current,
                    replacements,
                });
            }
        }
        Ok((edits, scanned, warnings))
    }

    /// Run Transform (+ Apply unless `dry_run`). On apply, a checkpoint is
    /// taken first; if any write fails the project is restored from it.
    pub fn run(
        &self,
        plan: &MigrationPlan,
        project: &Project,
        store: &mut CheckpointStore,
        opts: &RunOptions<'_>,
    ) -> Result<MigrationReport, MigrationError> {
        let (edits, files_scanned, warnings) = self.compute_edits(plan, project)?;

        let mut checkpoint_id = None;
        if !opts.dry_run && !edits.is_empty() {
            let cp = store.snapshot(
                opts.session_id,
                "pre-transform",
                &project.root,
                opts.snapshot_root,
            )?;
            checkpoint_id = Some(cp.id);
            for edit in &edits {
                let written = resolve_within(&project.root, &edit.path)
                    .and_then(|abs| project.fs.write(&abs, edit.updated.as_bytes()));
                if let Err(e) = written {
                    let rolled_back = store.restore(cp.id, &project.root).is_ok();
                    return Err(MigrationError::ApplyFailed {
                        path: edit.path.clone(),
                        cause: e.to_string(),
                        rolled_back,
                    });
                }
            }
        }

        Ok(MigrationReport {
            request: plan.request.clone(),
            dry_run: opts.dry_run,
            checkpoint_id,
            files_scanned,
            changes: edits
                .iter()
                .map(|e| FileChangeSummary {
                    path: e.path.clone(),
                    replacements: e.replacements.clone(),
                    diff: e.unified_diff(),
                })
                .collect(),
            skipped_mappings: plan.skipped.clone(),
            breaking_changes: plan.breaking_changes.clone(),
            recipes: plan.recipes.clone(),
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(from: &str, to: &str) -> RenameRule {
        RenameRule {
            from: from.into(),
            to: to.into(),
            kind: "rename".into(),
            confidence: 1.0,
            version: "1.21".into(),
        }
    }

    fn rewrite(src: &str, rules: &[RenameRule]) -> String {
        TokenRenameTransformer.transform(src, rules).0
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("1.21", "1.21.0"), Some(Ordering::Equal));
        assert_eq!(compare_versions("1.20.1", "1.21"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.9", "1.10"), Some(Ordering::Less));
        assert_eq!(compare_versions("1.21-pre1", "1.21"), None);
        assert!(in_upgrade_range("1.21", "1.20.1", "1.21"));
        assert!(!in_upgrade_range("1.20.1", "1.20.1", "1.21"));
    }

    #[test]
    fn test_whole_identifier_only() {
        let out = rewrite(
            "DemoWidget b = new DemoWidget(); DemoWidgetEntity e; MyDemoWidget m;",
            &[rule("DemoWidget", "DemoGadget")],
        );
        assert_eq!(
            out,
            "DemoGadget b = new DemoGadget(); DemoWidgetEntity e; MyDemoWidget m;"
        );
    }

    #[test]
    fn test_skips_strings_comments_and_text_blocks() {
        let src = "// DemoWidget here\n/* DemoWidget */ String s = \"DemoWidget\"; char c = 'D';\n\
                   String t = \"\"\"\n  DemoWidget\n\"\"\"; DemoWidget x;";
        let out = rewrite(src, &[rule("DemoWidget", "DemoGadget")]);
        assert!(out.contains("// DemoWidget here"));
        assert!(out.contains("/* DemoWidget */"));
        assert!(out.contains("\"DemoWidget\""));
        assert!(out.contains("\n  DemoWidget\n"));
        assert!(out.ends_with("DemoGadget x;"));
    }

    #[test]
    fn test_escaped_quote_does_not_end_string() {
        let out = rewrite(
            r#"s = "a\"DemoWidget"; DemoWidget y;"#,
            &[rule("DemoWidget", "DemoGadget")],
        );
        assert_eq!(out, r#"s = "a\"DemoWidget"; DemoGadget y;"#);
    }

    #[test]
    fn test_member_access_not_renamed_but_import_is() {
        let src = "import com.example.DemoWidget;\nfoo.DemoWidget = 1; DemoWidget.getId();";
        let out = rewrite(src, &[rule("DemoWidget", "DemoGadget")]);
        assert_eq!(
            out,
            "import com.example.DemoGadget;\nfoo.DemoWidget = 1; DemoGadget.getId();"
        );
    }

    #[test]
    fn test_qualified_rule_wins_over_simple() {
        let src = "import com.example.old.DemoWidget;\nDemoWidget b;";
        let rules = [
            rule("DemoWidget", "DemoGadget"),
            rule("com.example.old.DemoWidget", "com.example.demo.DemoWidget"),
        ];
        let (out, reps) = TokenRenameTransformer.transform(src, &rules);
        assert_eq!(out, "import com.example.demo.DemoWidget;\nDemoGadget b;");
        assert_eq!(reps.len(), 2);
        assert_eq!(reps[0].line, 1);
        assert_eq!(reps[1].line, 2);
    }

    #[test]
    fn test_numeric_literals_untouched() {
        let out = rewrite("int x = 0xBlock; float f = 1.5f;", &[rule("xBlock", "Y")]);
        assert_eq!(out, "int x = 0xBlock; float f = 1.5f;");
    }

    #[test]
    fn test_applies_to_java_and_kotlin_only() {
        let t = TokenRenameTransformer;
        assert!(t.applies_to(Path::new("src/A.java")));
        assert!(t.applies_to(Path::new("src/A.kt")));
        assert!(!t.applies_to(Path::new("build.gradle.kts")));
        assert!(!t.applies_to(Path::new("assets/lang/en_us.json")));
    }
}
