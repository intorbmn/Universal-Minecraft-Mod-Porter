//! `ummp recover` after a killed `migrate --apply`: files come back from the pre-transform
//! checkpoint and the interrupted job no longer claims to be `Running`.

use std::process::Command;

use ummp_core::app::jobs::INTERRUPTED;
use ummp_core::app::snapshots::{default_snapshots, save_store};
use ummp_core::checkpoint::CheckpointStore;
use ummp_core::jobs::{Job, JobState};
use ummp_core::persistence::JobDb;
use uuid::Uuid;

#[test]
fn recover_restores_files_and_marks_the_interrupted_job_failed() {
    let work = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let project = work.path().join("demo");
    let source = project.join("src/main/java/demo/DemoWidget.java");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    std::fs::write(project.join("build.gradle"), "plugins { id 'java' }\n").unwrap();
    std::fs::write(&source, "class DemoWidget {}\n").unwrap();

    // State a SIGKILL leaves behind: checkpoint persisted, job still Running, file half done.
    let session = Uuid::new_v4();
    let snaps = default_snapshots(&project);
    let mut store = CheckpointStore::new();
    store
        .snapshot(session, "before-migrate", &project, &snaps)
        .unwrap();
    save_store(&snaps, &store).unwrap();
    let mut job = Job::new(session, "migrate");
    job.mark_running();
    JobDb::open(&data.path().join("jobs.db"))
        .unwrap()
        .insert(&job)
        .unwrap();
    std::fs::write(&source, "class DemoGadget {}\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_ummp"))
        .env("UMMP_DATA_DIR", data.path())
        .args([
            "recover",
            project.to_str().unwrap(),
            "--session",
            &session.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        "class DemoWidget {}\n"
    );
    let jobs = JobDb::open(&data.path().join("jobs.db"))
        .unwrap()
        .by_session(session)
        .unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(
        jobs[0].state,
        JobState::Failed,
        "job left as {:?}",
        jobs[0].state
    );
    assert!(jobs[0].log.iter().any(|line| line == INTERRUPTED));
}
