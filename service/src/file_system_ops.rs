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

    /// Mock implementation for testing
    #[derive(Clone, Default)]
    pub struct MockFileSystemOps {
        existing_files: Arc<Mutex<HashSet<String>>>,
        deleted_files: Arc<Mutex<Vec<String>>>,
        fail_on_delete: Arc<Mutex<Option<String>>>,

        // TODO: unify, maybe existing files could be in entries?
        entries: Vec<Result<SimpleDirEntry, Error>>,
    }

    impl MockFileSystemOps {
        pub fn new() -> Self {
            Self::default()
        }

        /// Add a file to the mock file system
        pub fn add_file(&self, path: impl Into<String>) {
            self.existing_files.lock().unwrap().insert(path.into());
            println!(
                "Current existing files: {:?}",
                self.existing_files.lock().unwrap()
            );
        }

        /// Make deletion fail with a specific error message
        pub fn fail_delete_with(&self, error: impl Into<String>) {
            *self.fail_on_delete.lock().unwrap() = Some(error.into());
        }

        /// Get list of deleted files
        pub fn get_deleted_files(&self) -> Vec<String> {
            self.deleted_files.lock().unwrap().clone()
        }

        /// Check if a file was deleted
        pub fn was_deleted(&self, path: &str) -> bool {
            self.deleted_files
                .lock()
                .unwrap()
                .contains(&path.to_string())
        }

        /// Clear all state (useful between tests)
        pub fn clear(&self) {
            println!("Clearing mock file system state");
            self.existing_files.lock().unwrap().clear();
            self.deleted_files.lock().unwrap().clear();
            *self.fail_on_delete.lock().unwrap() = None;
        }

        pub fn add_entry(&mut self, entry: Result<SimpleDirEntry, Error>) {
            self.entries.push(entry);
        }
    }

    impl FileSystemOps for MockFileSystemOps {
        fn exists(&self, path: &Path) -> bool {
            println!("Checking existence of path: {}", path.display());
            println!(
                "Existing files in mock file system: {:?}",
                self.existing_files.lock().unwrap()
            );
            self.existing_files
                .lock()
                .unwrap()
                .contains(path.to_string_lossy().as_ref())
        }

        fn remove_file(&self, path: &Path) -> io::Result<()> {
            if let Some(error) = self.fail_on_delete.lock().unwrap().as_ref() {
                return Err(io::Error::other(error.clone()));
            }

            let path_str = path.to_string_lossy().to_string();
            self.deleted_files.lock().unwrap().push(path_str.clone());
            self.existing_files.lock().unwrap().remove(&path_str);
            Ok(())
        }

        fn is_zip_archive(&self, path: &Path) -> Result<bool, Error> {
            println!("Checking if path is zip archive: {}", path.display());
            let path_str = path.to_string_lossy();
            println!("Path string: {}", path_str);
            if !self
                .existing_files
                .lock()
                .unwrap()
                .contains(path_str.as_ref())
            {
                println!("File does not exist in mock file system: {}", path_str,);
                Err(Error::IoError(format!("File does not exist: {}", path_str)))
            } else {
                Ok(path_str.ends_with(".zip"))
            }
        }

        fn move_file(&self, from: &Path, to: &Path) -> io::Result<()> {
            let from_str = from.to_string_lossy().to_string();
            let to_str = to.to_string_lossy().to_string();

            if !self.existing_files.lock().unwrap().contains(&from_str) {
                return Err(io::Error::other(format!(
                    "Source file does not exist: {}",
                    from_str
                )));
            }

            self.existing_files.lock().unwrap().remove(&from_str);
            self.existing_files.lock().unwrap().insert(to_str);
            Ok(())
        }

        fn read_dir(
            &self,
            _path: &Path, // if needed could be mapped to entries
        ) -> io::Result<Box<dyn Iterator<Item = Result<SimpleDirEntry, Error>>>> {
            Ok(Box::new(self.entries.clone().into_iter()))
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
