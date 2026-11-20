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
