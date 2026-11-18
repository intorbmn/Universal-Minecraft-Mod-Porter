//! Infrastructure: checkpoint / rollback for migration sessions.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: Uuid,
    pub session_id: Uuid,
    pub step: String,
    pub snapshot_path: PathBuf,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckpointStore {
    pub checkpoints: Vec<Checkpoint>,
}

impl CheckpointStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, checkpoint: Checkpoint) {
        self.checkpoints.push(checkpoint);
    }

    pub fn latest(&self) -> Option<&Checkpoint> {
        self.checkpoints.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkpoint_store() {
        let session_id = Uuid::new_v4();
        let mut store = CheckpointStore::new();

        let cp1 = Checkpoint {
            id: Uuid::new_v4(),
            session_id,
            step: "import".into(),
            snapshot_path: PathBuf::from("snap1"),
            created_at: chrono::Utc::now(),
        };
        let cp2 = Checkpoint {
            id: Uuid::new_v4(),
            session_id,
            step: "transform".into(),
            snapshot_path: PathBuf::from("snap2"),
            created_at: chrono::Utc::now(),
        };

        store.add(cp1);
        assert_eq!(store.latest().unwrap().step, "import");
        store.add(cp2);
        assert_eq!(store.latest().unwrap().step, "transform");
    }
}
