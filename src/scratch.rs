//! A temporary directory that deletes itself, for the tests.

use std::path::{Path, PathBuf};

/// A folder in `temp_dir()` that removes itself when it goes out of scope.
pub struct TempDir(PathBuf);

impl TempDir {
    /// Creates the folder, deleting whatever exists with the same name.
    pub fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temporary directory");
        Self(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl std::ops::Deref for TempDir {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
