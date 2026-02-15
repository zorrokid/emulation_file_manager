//! File system operations abstraction for testing
//!
//! This module provides a trait-based abstraction over file system operations,
//! allowing services to be tested without touching the real file system.
//!
//! # Usage in Production
//!
//! ```rust,ignore
//! use service::file_system_ops::{FileSystemOps, StdFileSystemOps};
//! use service::file_set_deletion_service::FileSetDeletionService;
//!
//! // Use the default implementation (StdFileSystemOps)
//! let service = FileSetDeletionService::new(repo_manager, settings);
//! ```
//!
//! # Usage in Tests
//!
//! ```rust,ignore
//! use service::file_system_ops::mock::MockFileSystemOps;
//! use service::file_set_deletion_service::FileSetDeletionService;
//!
//! let mock_fs = Arc::new(MockFileSystemOps::new());
//! mock_fs.add_file("/test/rom/game.zst");
//!
//! let service = FileSetDeletionService::new_with_fs_ops(
//!     repo_manager,
//!     settings,
//!     mock_fs.clone(),
//! );
//!
//! // Call your service methods...
//!
//! // Verify the mock's state
//! assert!(mock_fs.was_deleted("/test/rom/game.zst"));
//! ```

use std::io;
use std::path::{Path, PathBuf};

use utils::file_util;

use crate::error::Error;

#[derive(Debug, Clone)]
pub struct SimpleDirEntry {
    pub path: PathBuf,
}

/// Trait for file system operations to enable testing
pub trait FileSystemOps: Send + Sync {
    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Remove a file at the given path
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Check if a file is a zip archive
    fn is_zip_archive(&self, path: &Path) -> Result<bool, Error>;

    /// Move a file from one path to another
    fn move_file(&self, from: &Path, to: &Path) -> io::Result<()>;

    // For easier mocking we use our own SimpleDirEntry instead of std::fs::DirEntry and return
    // boxed iterator to avoid associated type complications.
    fn read_dir(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = Result<SimpleDirEntry, Error>>>>;
}

/// Production implementation using std::fs
#[derive(Debug, Clone, Copy)]
pub struct StdFileSystemOps;

impl FileSystemOps for StdFileSystemOps {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn is_zip_archive(&self, path: &Path) -> Result<bool, Error> {
        let res = file_util::is_zip_file(path);
        res.map_err(|e| Error::IoError(format!("Failed to check if file is zip archive: {}", e)))
    }

    fn move_file(&self, from: &Path, to: &Path) -> io::Result<()> {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?
        }
        std::fs::rename(from, to)
    }

    fn read_dir(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = Result<SimpleDirEntry, Error>>>> {
        let iter = std::fs::read_dir(path)?;
        Ok(Box::new(iter.map(|res| {
            res.map_err(Error::from)
                .map(|entry| SimpleDirEntry { path: entry.path() })
        })))
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex};

    /// Internal state for MockFileSystemOps.
    ///
    /// Groups all mutable state into a single struct for simplified locking.
    #[derive(Default)]
    struct MockState {
        existing_files: HashSet<String>,
        deleted_files: Vec<String>,
        fail_on_delete: Option<String>,
        // TODO: unify, maybe existing files could be in entries?
        entries: Vec<Result<SimpleDirEntry, Error>>,
    }

    /// Mock implementation for testing
    #[derive(Clone)]
    pub struct MockFileSystemOps {
        state: Arc<Mutex<MockState>>,
    }

    impl Default for MockFileSystemOps {
        fn default() -> Self {
            Self {
                state: Arc::new(Mutex::new(MockState::default())),
            }
        }
    }

    impl MockFileSystemOps {
        pub fn new() -> Self {
            Self::default()
        }

        /// Add a file to the mock file system
        pub fn add_file(&self, path: impl Into<String>) {
            let mut state = self.state.lock().unwrap();
            state.existing_files.insert(path.into());
            println!("Current existing files: {:?}", state.existing_files);
        }

        /// Make deletion fail with a specific error message
        pub fn fail_delete_with(&self, error: impl Into<String>) {
            let mut state = self.state.lock().unwrap();
            state.fail_on_delete = Some(error.into());
        }

        /// Get list of deleted files
        pub fn get_deleted_files(&self) -> Vec<String> {
            let state = self.state.lock().unwrap();
            state.deleted_files.clone()
        }

        /// Check if a file was deleted
        pub fn was_deleted(&self, path: &str) -> bool {
            let state = self.state.lock().unwrap();
            state.deleted_files.contains(&path.to_string())
        }

        /// Clear all state (useful between tests)
        pub fn clear(&self) {
            println!("Clearing mock file system state");
            let mut state = self.state.lock().unwrap();
            *state = MockState::default();
        }

        pub fn add_entry(&mut self, entry: Result<SimpleDirEntry, Error>) {
            let mut state = self.state.lock().unwrap();
            state.entries.push(entry);
        }
    }

    impl FileSystemOps for MockFileSystemOps {
        fn exists(&self, path: &Path) -> bool {
            let state = self.state.lock().unwrap();
            println!("Checking existence of path: {}", path.display());
            println!("Existing files in mock file system: {:?}", state.existing_files);
            state.existing_files.contains(path.to_string_lossy().as_ref())
        }

        fn remove_file(&self, path: &Path) -> io::Result<()> {
            let mut state = self.state.lock().unwrap();

            if let Some(error) = state.fail_on_delete.as_ref() {
                return Err(io::Error::other(error.clone()));
            }

            let path_str = path.to_string_lossy().to_string();
            state.deleted_files.push(path_str.clone());
            state.existing_files.remove(&path_str);
            Ok(())
        }

        fn is_zip_archive(&self, path: &Path) -> Result<bool, Error> {
            let state = self.state.lock().unwrap();
            println!("Checking if path is zip archive: {}", path.display());
            let path_str = path.to_string_lossy();
            println!("Path string: {}", path_str);

            if !state.existing_files.contains(path_str.as_ref()) {
                println!("File does not exist in mock file system: {}", path_str,);
                Err(Error::IoError(format!("File does not exist: {}", path_str)))
            } else {
                Ok(path_str.ends_with(".zip"))
            }
        }

        fn move_file(&self, from: &Path, to: &Path) -> io::Result<()> {
            let mut state = self.state.lock().unwrap();
            let from_str = from.to_string_lossy().to_string();
            let to_str = to.to_string_lossy().to_string();

            if !state.existing_files.contains(&from_str) {
                return Err(io::Error::other(format!(
                    "Source file does not exist: {}",
                    from_str
                )));
            }

            state.existing_files.remove(&from_str);
            state.existing_files.insert(to_str);
            Ok(())
        }

        fn read_dir(
            &self,
            _path: &Path, // if needed could be mapped to entries
        ) -> io::Result<Box<dyn Iterator<Item = Result<SimpleDirEntry, Error>>>> {
            let state = self.state.lock().unwrap();
            Ok(Box::new(state.entries.clone().into_iter()))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::file_system_ops::mock::MockFileSystemOps;

    use super::*;

    #[test]
    fn test_mock_file_system_ops() {
        let mock_fs = MockFileSystemOps::new();
        mock_fs.add_file("/test/file1.txt");
        assert!(mock_fs.exists(Path::new("/test/file1.txt")));
        assert!(!mock_fs.exists(Path::new("/test/file2.txt")));
        mock_fs.remove_file(Path::new("/test/file1.txt")).unwrap();
        assert!(!mock_fs.exists(Path::new("/test/file1.txt")));

        mock_fs.add_file("/test/archive.zip");
        let is_zip = mock_fs
            .is_zip_archive(Path::new("/test/archive.zip"))
            .unwrap();
        assert!(is_zip);

        mock_fs.add_file("/test/move_source.txt");
        mock_fs
            .move_file(
                Path::new("/test/move_source.txt"),
                Path::new("/test/move_dest.txt"),
            )
            .unwrap();
        assert!(!mock_fs.exists(Path::new("/test/move_source.txt")));
        assert!(mock_fs.exists(Path::new("/test/move_dest.txt")));

        mock_fs.fail_delete_with("Simulated delete failure");
        let result = mock_fs.remove_file(Path::new("/test/move_dest.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_read_dir_mock() {
        let mut mock_fs = MockFileSystemOps::new();
        mock_fs.add_entry(Ok(SimpleDirEntry {
            path: PathBuf::from("/test/file1.txt"),
        }));
        mock_fs.add_entry(Ok(SimpleDirEntry {
            path: PathBuf::from("/test/file2.txt"),
        }));

        let entries: Vec<_> = mock_fs.read_dir(Path::new("/test")).unwrap().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].as_ref().unwrap().path,
            PathBuf::from("/test/file1.txt")
        );
        assert_eq!(
            entries[1].as_ref().unwrap().path,
            PathBuf::from("/test/file2.txt")
        );
    }
}
