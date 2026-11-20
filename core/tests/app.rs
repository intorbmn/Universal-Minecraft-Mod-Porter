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
