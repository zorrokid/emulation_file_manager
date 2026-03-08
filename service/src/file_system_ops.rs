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

use std::fs::read_dir;
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

    /// Check if a directory is accessible (exists, is a directory, and can be read)
    fn is_accesssible_dir(&self, path: &Path) -> bool;

    fn is_file(&self, path: &Path) -> bool;
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

    fn is_accesssible_dir(&self, path: &Path) -> bool {
        if !path.exists() || !path.is_dir() {
            return false;
        }

        // Do a read check if the dir is actually mounted and accessible:
        match read_dir(path) {
            Ok(mut entries) => entries.next().is_some(),
            Err(_) => false,
        }
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A file entry in the mock file system.
    ///
    /// Each entry can be either a successful file/directory or an error result from read_dir.
    /// This is the single source of truth for all file system operations.
    #[derive(Clone)]
    enum MockFileEntry {
        /// A file or directory that exists
        File { path: PathBuf, is_file: bool },
        /// An error that occurs when reading this entry from a directory
        ReadError(Error),
    }

    /// Internal state for MockFileSystemOps.
    ///
    /// Single source of truth: `entries` vec contains all file system state.
    /// Each entry is either a valid file/dir or represents a read error.
    #[derive(Default)]
    struct MockState {
        /// Single source of truth: all file entries (files, dirs, and read errors)
        entries: Vec<MockFileEntry>,
        deleted_files: Vec<String>,
        fail_on_delete: Option<String>,
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
            let path_str = path.into();
            let mut state = self.state.lock().unwrap();
            state.entries.push(MockFileEntry::File {
                path: PathBuf::from(&path_str),
                is_file: true,
            });
        }

        /// Add a directory to the mock file system
        pub fn add_dir(&self, path: impl Into<String>) {
            let path_str = path.into();
            let mut state = self.state.lock().unwrap();
            state.entries.push(MockFileEntry::File {
                path: PathBuf::from(&path_str),
                is_file: false,
            });
        }

        /// Add an entry result (file or error) to the mock file system
        ///
        /// This is the most flexible method:
        /// - `Ok(SimpleDirEntry { path })` adds a file
        /// - `Err(error)` adds a read error
        ///
        /// Useful for simulating complex read_dir scenarios with mixed success/failure.
        pub fn add_entry(&self, entry: Result<SimpleDirEntry, Error>) {
            let mut state = self.state.lock().unwrap();
            match entry {
                Ok(dir_entry) => {
                    // Infer is_file from extension (heuristic)
                    let is_file = dir_entry
                        .path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some();
                    state.entries.push(MockFileEntry::File {
                        path: dir_entry.path,
                        is_file,
                    });
                }
                Err(error) => {
                    state.entries.push(MockFileEntry::ReadError(error));
                }
            }
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
            let mut state = self.state.lock().unwrap();
            *state = MockState::default();
        }
    }

    impl FileSystemOps for MockFileSystemOps {
        fn exists(&self, path: &Path) -> bool {
            let state = self.state.lock().unwrap();
            let path_str = path.to_string_lossy();
            state.entries.iter().any(|entry| match entry {
                MockFileEntry::File { path: p, .. } => p.to_string_lossy() == path_str,
                MockFileEntry::ReadError(_) => false,
            })
        }

        fn remove_file(&self, path: &Path) -> io::Result<()> {
            let mut state = self.state.lock().unwrap();

            if let Some(error) = state.fail_on_delete.as_ref() {
                return Err(io::Error::other(error.clone()));
            }

            let path_str = path.to_string_lossy().to_string();

            // Find and remove the entry
            if let Some(pos) = state.entries.iter().position(|entry| {
                matches!(entry, MockFileEntry::File { path: p, .. } if p.to_string_lossy() == path_str)
            }) {
                state.entries.remove(pos);
                state.deleted_files.push(path_str);
                Ok(())
            } else {
                Err(io::Error::other(format!("File does not exist: {}", path_str)))
            }
        }

        fn is_zip_archive(&self, path: &Path) -> Result<bool, Error> {
            let state = self.state.lock().unwrap();
            let path_str = path.to_string_lossy();

            if state.entries.iter().any(|entry| {
                matches!(entry, MockFileEntry::File { path: p, .. } if p.to_string_lossy() == path_str)
            }) {
                Ok(path_str.ends_with(".zip"))
            } else {
                Err(Error::IoError(format!("File does not exist: {}", path_str)))
            }
        }

        fn move_file(&self, from: &Path, to: &Path) -> io::Result<()> {
            let mut state = self.state.lock().unwrap();
            let from_str = from.to_string_lossy().to_string();
            let to_str = to.to_string_lossy().to_string();

            // Find the entry to move
            if let Some(pos) = state.entries.iter().position(|entry| {
                matches!(entry, MockFileEntry::File { path: p, .. } if p.to_string_lossy() == from_str)
            }) {
                if let MockFileEntry::File { is_file, .. } = &state.entries[pos] {
                    let is_file = *is_file;
                    state.entries[pos] = MockFileEntry::File {
                        path: PathBuf::from(&to_str),
                        is_file,
                    };
                    Ok(())
                } else {
                    Err(io::Error::other("Invalid entry type"))
                }
            } else {
                Err(io::Error::other(format!(
                    "Source file does not exist: {}",
                    from_str
                )))
            }
        }

        fn read_dir(
            &self,
            path: &Path,
        ) -> io::Result<Box<dyn Iterator<Item = Result<SimpleDirEntry, Error>>>> {
            let state = self.state.lock().unwrap();
            let path_str = path.to_string_lossy().to_string();

            // Return all entries in or under this path
            let entries: Vec<_> = state
                .entries
                .iter()
                .filter(|entry| {
                    if let MockFileEntry::File { path: p, .. } = entry {
                        let file_path_str = p.to_string_lossy();
                        file_path_str.starts_with(&path_str)
                    } else {
                        // Include read errors in the results
                        true
                    }
                })
                .map(|entry| match entry {
                    MockFileEntry::File { path: p, .. } => {
                        Ok(SimpleDirEntry { path: p.clone() })
                    }
                    MockFileEntry::ReadError(err) => Err(err.clone()),
                })
                .collect();

            Ok(Box::new(entries.into_iter()))
        }

        fn is_accesssible_dir(&self, _: &Path) -> bool {
            // For testing purposes, we can assume all directories are accessible.
            true
        }

        fn is_file(&self, path: &Path) -> bool {
            let state = self.state.lock().unwrap();
            let path_str = path.to_string_lossy();
            state.entries.iter().any(|entry| {
                matches!(entry, MockFileEntry::File { path: p, is_file: true } if p.to_string_lossy() == path_str)
            })
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

        // Add and check file existence
        mock_fs.add_file("/test/file1.txt");
        assert!(mock_fs.exists(Path::new("/test/file1.txt")));
        assert!(!mock_fs.exists(Path::new("/test/file2.txt")));

        // Remove file
        mock_fs.remove_file(Path::new("/test/file1.txt")).unwrap();
        assert!(!mock_fs.exists(Path::new("/test/file1.txt")));

        // Test zip detection
        mock_fs.add_file("/test/archive.zip");
        let is_zip = mock_fs
            .is_zip_archive(Path::new("/test/archive.zip"))
            .unwrap();
        assert!(is_zip);

        // Test move file
        mock_fs.add_file("/test/move_source.txt");
        mock_fs
            .move_file(
                Path::new("/test/move_source.txt"),
                Path::new("/test/move_dest.txt"),
            )
            .unwrap();
        assert!(!mock_fs.exists(Path::new("/test/move_source.txt")));
        assert!(mock_fs.exists(Path::new("/test/move_dest.txt")));

        // Test delete failure
        mock_fs.fail_delete_with("Simulated delete failure");
        let result = mock_fs.remove_file(Path::new("/test/move_dest.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_read_dir_mock() {
        let mock_fs = MockFileSystemOps::new();

        // Add files to the mock file system using single API
        mock_fs.add_file("/test/file1.txt");
        mock_fs.add_file("/test/file2.txt");

        // read_dir should return files in the directory
        let entries: Vec<_> = mock_fs.read_dir(Path::new("/test")).unwrap().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_is_file_mock() {
        let mock_fs = MockFileSystemOps::new();

        mock_fs.add_file("/test/myfile.txt");
        mock_fs.add_dir("/test/mydir");

        // Check file and directory detection
        assert!(mock_fs.is_file(Path::new("/test/myfile.txt")));
        assert!(!mock_fs.is_file(Path::new("/test/mydir")));
        assert!(!mock_fs.is_file(Path::new("/test/nonexistent")));
    }

    #[test]
    fn test_read_dir_with_errors() {
        let mock_fs = MockFileSystemOps::new();

        // Add mixed success and error entries
        mock_fs.add_entry(Ok(SimpleDirEntry {
            path: PathBuf::from("/test/file1.txt"),
        }));
        mock_fs.add_entry(Err(Error::IoError("Simulated read failure".to_string())));
        mock_fs.add_entry(Ok(SimpleDirEntry {
            path: PathBuf::from("/test/file2.txt"),
        }));

        let entries: Vec<_> = mock_fs.read_dir(Path::new("/test")).unwrap().collect();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_ok());
        assert!(entries[1].is_err());
        assert!(entries[2].is_ok());
    }
}
