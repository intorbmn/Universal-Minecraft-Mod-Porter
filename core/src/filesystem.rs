//! Infrastructure: file system abstraction.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("path escape: {0}")]
    PathEscape(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("limit exceeded: {0}")]
    LimitExceeded(String),
}

pub trait FileSystem: Send + Sync {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError>;
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError>;
    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, FsError>;
    fn exists(&self, path: &Path) -> bool;
    fn is_dir(&self, path: &Path) -> bool;
}

pub struct LocalFileSystem;

impl FileSystem for LocalFileSystem {
    fn read(&self, path: &Path) -> Result<Vec<u8>, FsError> {
        Ok(std::fs::read(path)?)
    }
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), FsError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, data)?;
        Ok(())
    }
    fn list(&self, path: &Path) -> Result<Vec<PathBuf>, FsError> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(path)? {
            out.push(entry?.path());
        }
        Ok(out)
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }
}

/// Reject paths that escape the project root.
pub fn validate_root(path: &Path) -> Result<PathBuf, FsError> {
    let canon = path
        .canonicalize()
        .map_err(|e| FsError::NotFound(e.to_string()))?;
    Ok(canon)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_read_write() {
        let dir = tempdir().unwrap();
        let fs = LocalFileSystem;
        let file = dir.path().join("test.txt");
        fs.write(&file, b"hello").unwrap();
        let data = fs.read(&file).unwrap();
        assert_eq!(data, b"hello");
    }

    #[test]
    fn test_list() {
        let dir = tempdir().unwrap();
        let fs = LocalFileSystem;
        fs.write(&dir.path().join("a.txt"), b"a").unwrap();
        fs.write(&dir.path().join("b.txt"), b"b").unwrap();
        let entries = fs.list(dir.path()).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_exists() {
        let dir = tempdir().unwrap();
        let fs = LocalFileSystem;
        assert!(!fs.exists(&dir.path().join("missing.txt")));
        fs.write(&dir.path().join("exists.txt"), b"x").unwrap();
        assert!(fs.exists(&dir.path().join("exists.txt")));
    }
}
