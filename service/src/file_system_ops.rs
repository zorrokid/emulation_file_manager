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
use std::path::Path;

use utils::file_util;

use crate::error::Error;

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
    }
}
