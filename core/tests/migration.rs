//! Phase 6 regression tests: plan → transform → diff → apply → rollback on
//! the `tests/fixtures/forge-src-1.16` corpus project.
//!
//! The mappings below are test data describing the Forge 1.16.5 (MCP) →
//! 1.17 (official names) package moves; they are not knowledge-base seed data.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use ummp_core::checkpoint::{collect_files, CheckpointStore};
use ummp_core::filesystem::{FileSystem, FsError, LocalFileSystem};
use ummp_core::knowledge::{BreakingChange, KnowledgeBase, Mapping};
use ummp_core::migration::{
    MigrationEngine, MigrationError, MigrationPlanner, MigrationRequest, RunOptions,
};
use ummp_core::project::{DefaultProjectManager, Project, ProjectManager};
use uuid::Uuid;

const FIXTURE: &str = "../tests/fixtures/forge-src-1.16";
const GOLDEN: &str = "../tests/fixtures/golden/ExampleMod-1.17.java.golden";
const MOD_JAVA: &str = "src/main/java/com/example/examplemod/ExampleMod.java";

fn copy_tree(from: &Path, to: &Path) {
    for rel in collect_files(from).unwrap() {
        let dest = to.join(&rel);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(from.join(&rel), dest).unwrap();
    }
}

fn read_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    collect_files(root)
        .unwrap()
        .into_iter()
        .map(|rel| {
            let bytes = fs::read(root.join(&rel)).unwrap();
            (rel, bytes)
        })
        .collect()
}

fn mapping(version: &str, from: &str, to: &str, confidence: f64) -> Mapping {
    Mapping {
        version: version.into(),
        loader: "forge".into(),
        from_api: from.into(),
        to_api: to.into(),
        kind: "move".into(),
        source: "test".into(),
        confidence,
    }
}

fn forge_117_kb(dir: &Path) -> KnowledgeBase {
    let kb = KnowledgeBase::open(&dir.join("kb.db")).unwrap();
    for m in [
        mapping(
            "1.17",
            "net.minecraft.block.Block",
            "net.minecraft.world.level.block.Block",
            0.95,
        ),
        mapping(
            "1.17",
            "net.minecraft.block.AbstractBlock",
            "net.minecraft.world.level.block.state.BlockBehaviour",
            0.95,
        ),
        mapping("1.17", "AbstractBlock", "BlockBehaviour", 0.9),
        mapping(
            "1.17",
            "net.minecraft.block.material.Material",
            "net.minecraft.world.level.material.Material",
            0.95,
        ),
        mapping(
            "1.17",
            "net.minecraft.item.Item",
            "net.minecraft.world.item.Item",
            0.95,
        ),
        mapping(
            "1.17",
            "net.minecraft.item.BlockItem",
            "net.minecraft.world.item.BlockItem",
            0.95,
        ),
        // Below the threshold: must be reported, never applied.
        mapping("1.17", "EXAMPLE_BLOCK", "SHOULD_NOT_APPEAR", 0.4),
        // Outside the (1.16.5, 1.17.1] range: ignored entirely.
        mapping("1.20", "Item", "SHOULD_NOT_APPEAR_EITHER", 0.99),
    ] {
        kb.insert_mapping(&m).unwrap();
    }
    kb.insert_breaking_change(&BreakingChange {
        version: "1.17".into(),
        change: "Java 16 required".into(),
        severity: "high".into(),
        workaround: Some("Update the Gradle toolchain to Java 16+".into()),
        source: "test".into(),
    })
    .unwrap();
    kb
}

fn request() -> MigrationRequest {
    MigrationRequest {
        source_minecraft: "1.16.5".into(),
        target_minecraft: "1.17.1".into(),
        target_loader: "forge".into(),
        min_confidence: 0.8,
    }
}

/// Phase 6 acceptance: a corpus mod migrates through the transform step and
/// produces a valid diff; rollback restores the original exactly.
#[test]
fn test_forge_116_to_117_dry_run_apply_rollback() {
    let work = tempfile::tempdir().unwrap();
    let snaps = tempfile::tempdir().unwrap();
    let kb_dir = tempfile::tempdir().unwrap();
    copy_tree(Path::new(FIXTURE), work.path());
    let original = read_tree(work.path());

    let kb = forge_117_kb(kb_dir.path());
    let plan = MigrationPlanner::plan(&kb, &request()).unwrap();
    assert_eq!(plan.stages.len(), 1);
    assert_eq!(plan.rule_count(), 6);
    assert_eq!(plan.skipped.len(), 1);
    assert_eq!(plan.breaking_changes.len(), 1);

    let project = DefaultProjectManager.open(work.path()).unwrap();
    let engine = MigrationEngine::default();
    let mut store = CheckpointStore::new();

    // Dry run: report + diff, nothing written.
    let dry = engine
        .run(
            &plan,
            &project,
            &mut store,
            &RunOptions {
                dry_run: true,
                snapshot_root: snaps.path(),
                session_id: Uuid::new_v4(),
            },
        )
        .unwrap();
    assert_eq!(read_tree(work.path()), original);
    assert!(dry.checkpoint_id.is_none());
    assert_eq!(dry.changes.len(), 1);
    assert_eq!(dry.total_replacements(), 6);
    let diff = &dry.changes[0].diff;
    assert!(diff.starts_with(&format!("--- a/{MOD_JAVA}\n+++ b/{MOD_JAVA}\n")));
    assert!(diff.contains("-import net.minecraft.block.Block;\n"));
    assert!(diff.contains("+import net.minecraft.world.level.block.Block;\n"));
    let md = dry.to_markdown();
    assert!(md.contains("Java 16 required"));
    assert!(md.contains("EXAMPLE_BLOCK"));
    assert!(dry.to_json().unwrap().contains("\"dry_run\": true"));

    // Apply: matches the golden file; resources untouched.
    let applied = engine
        .run(
            &plan,
            &project,
            &mut store,
            &RunOptions {
                dry_run: false,
                snapshot_root: snaps.path(),
                session_id: Uuid::new_v4(),
            },
        )
        .unwrap();
    let cp_id = applied.checkpoint_id.expect("apply must checkpoint first");
    let migrated = fs::read_to_string(work.path().join(MOD_JAVA)).unwrap();
    let golden = fs::read_to_string(GOLDEN).unwrap();
    assert_eq!(migrated.replace("\r\n", "\n"), golden.replace("\r\n", "\n"));
    assert!(!migrated.contains("SHOULD_NOT_APPEAR"));
    let lang = "src/main/resources/assets/examplemod/lang/en_us.json";
    assert_eq!(
        fs::read(work.path().join(lang)).unwrap(),
        original[Path::new(lang)]
    );

    // Rollback: byte-identical to the original tree.
    store.restore(cp_id, &project.root).unwrap();
    assert_eq!(read_tree(work.path()), original);
}

#[test]
fn test_stages_run_in_version_order() {
    let kb_dir = tempfile::tempdir().unwrap();
    let kb = KnowledgeBase::open(&kb_dir.path().join("kb.db")).unwrap();
    // Inserted out of order on purpose.
    kb.insert_mapping(&mapping("1.18", "Mid", "Final", 1.0))
        .unwrap();
    kb.insert_mapping(&mapping("1.17", "Old", "Mid", 1.0))
        .unwrap();

    let work = tempfile::tempdir().unwrap();
    fs::write(work.path().join("A.java"), "class A { Old o; }").unwrap();
    let project = DefaultProjectManager.open(work.path()).unwrap();

    let mut req = request();
    req.target_minecraft = "1.18.2".into();
    let plan = MigrationPlanner::plan(&kb, &req).unwrap();
    let versions: Vec<&str> = plan.stages.iter().map(|s| s.version.as_str()).collect();
    assert_eq!(versions, ["1.17", "1.18"]);

    let (edits, _, _) = MigrationEngine::default()
        .compute_edits(&plan, &project)
        .unwrap();
    assert_eq!(edits[0].updated, "class A { Final o; }");
}

#[test]
fn test_conflicting_mappings_are_not_applied() {
    let kb_dir = tempfile::tempdir().unwrap();
    let kb = KnowledgeBase::open(&kb_dir.path().join("kb.db")).unwrap();
    kb.insert_mapping(&mapping("1.17", "Thing", "ThingA", 1.0))
        .unwrap();
    kb.insert_mapping(&mapping("1.17", "Thing", "ThingB", 1.0))
        .unwrap();
    let plan = MigrationPlanner::plan(&kb, &request()).unwrap();
    assert_eq!(plan.rule_count(), 0);
    assert_eq!(plan.skipped.len(), 2);
}

#[test]
fn test_downgrade_and_non_numeric_versions_rejected() {
    let kb_dir = tempfile::tempdir().unwrap();
    let kb = KnowledgeBase::open(&kb_dir.path().join("kb.db")).unwrap();
    let mut req = request();
    req.target_minecraft = "1.12.2".into();
    assert!(matches!(
        MigrationPlanner::plan(&kb, &req),
        Err(MigrationError::InvalidRequest(_))
    ));
    req.target_minecraft = "latest".into();
    assert!(matches!(
        MigrationPlanner::plan(&kb, &req),
        Err(MigrationError::InvalidRequest(_))
    ));
}

/// Succeeds for the first `ok_writes` writes, then fails.
struct FailingFs {
    ok_writes: usize,
    writes: AtomicUsize,
}

impl FileSystem for FailingFs {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        LocalFileSystem.read(path)
    }
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        if self.writes.fetch_add(1, Ordering::SeqCst) >= self.ok_writes {
            return Err(FsError::LimitExceeded("simulated disk full".into()));
        }
        LocalFileSystem.write(path, data)
    }
    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, FsError> {
        LocalFileSystem.list(path)
    }
    fn exists(&self, path: &Path) -> bool {
        LocalFileSystem.exists(path)
    }
    fn is_dir(&self, path: &Path) -> bool {
        LocalFileSystem.is_dir(path)
    }
}

#[test]
fn test_failed_apply_rolls_back_partial_writes() {
    let work = tempfile::tempdir().unwrap();
    let snaps = tempfile::tempdir().unwrap();
    let kb_dir = tempfile::tempdir().unwrap();
    fs::write(work.path().join("A.java"), "class A { Old o; }").unwrap();
    fs::write(work.path().join("B.java"), "class B { Old o; }").unwrap();
    let original = read_tree(work.path());

    let kb = KnowledgeBase::open(&kb_dir.path().join("kb.db")).unwrap();
    kb.insert_mapping(&mapping("1.17", "Old", "New", 1.0))
        .unwrap();
    let plan = MigrationPlanner::plan(&kb, &request()).unwrap();

    let project = Project {
        root: work.path().canonicalize().unwrap(),
        fs: Arc::new(FailingFs {
            ok_writes: 1,
            writes: AtomicUsize::new(0),
        }),
    };
    let err = MigrationEngine::default()
        .run(
            &plan,
            &project,
            &mut CheckpointStore::new(),
            &RunOptions {
                dry_run: false,
                snapshot_root: snaps.path(),
                session_id: Uuid::new_v4(),
            },
        )
        .unwrap_err();

    assert!(matches!(
        err,
        MigrationError::ApplyFailed {
            rolled_back: true,
            ..
        }
    ));
    assert_eq!(read_tree(work.path()), original);
}
