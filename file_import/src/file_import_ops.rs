use core_types::{ReadFile, Sha1Checksum};
use std::{collections::HashMap, path::Path};

use crate::FileImportError;

/// Trait for file import operations to enable testing
///
/// This trait abstracts file reading and checksum calculation operations,
/// allowing them to be mocked in tests.
pub trait FileImportOps: Send + Sync {
    /// Read contents of a ZIP archive and calculate checksums for each file
    ///
    /// Returns a map of SHA1 checksums to file information for all files in the archive.
    fn read_zip_contents_with_checksums(
        &self,
        file_path: &Path,
    ) -> Result<HashMap<Sha1Checksum, ReadFile>, FileImportError>;

    /// Read a single file and calculate its checksum
    ///
    /// Returns a map with a single entry containing the file's SHA1 checksum and metadata.
    fn read_file_checksum(
        &self,
        file_path: &Path,
    ) -> Result<HashMap<Sha1Checksum, ReadFile>, FileImportError>;
}

/// Standard implementation using actual file system operations
#[derive(Debug, Clone)]
pub struct StdFileImportOps;

impl FileImportOps for StdFileImportOps {
    fn read_zip_contents_with_checksums(
        &self,
        file_path: &Path,
    ) -> Result<HashMap<Sha1Checksum, ReadFile>, FileImportError> {
        crate::read_zip_contents_with_checksums(&file_path.to_path_buf())
    }

    fn read_file_checksum(
        &self,
        file_path: &Path,
    ) -> Result<HashMap<Sha1Checksum, ReadFile>, FileImportError> {
        crate::read_file_checksum(&file_path.to_path_buf())
    }
}

pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock implementation for testing file import operations
    #[derive(Clone, Default)]
    pub struct MockFileImportOps {
        zip_contents: Arc<Mutex<HashMap<Sha1Checksum, ReadFile>>>,
        file_checksums: Arc<Mutex<HashMap<Sha1Checksum, ReadFile>>>,
        should_fail: Arc<Mutex<bool>>,
    }

    impl MockFileImportOps {
        pub fn new() -> Self {
            Self::default()
        }

        /// Add a file entry to be returned by read_zip_contents_with_checksums
        pub fn add_zip_file(&self, checksum: Sha1Checksum, read_file: ReadFile) {
            self.zip_contents
                .lock()
                .unwrap()
                .insert(checksum, read_file);
        }

        /// Add a file entry to be returned by read_file_checksum
        pub fn add_file_checksum(&self, checksum: Sha1Checksum, read_file: ReadFile) {
            self.file_checksums
                .lock()
                .unwrap()
                .insert(checksum, read_file);
        }

        /// Make all operations fail with an error
        pub fn set_should_fail(&self, should_fail: bool) {
            *self.should_fail.lock().unwrap() = should_fail;
        }
    }

    impl FileImportOps for MockFileImportOps {
        fn read_zip_contents_with_checksums(
            &self,
            _file_path: &Path,
        ) -> Result<HashMap<Sha1Checksum, ReadFile>, FileImportError> {
            if *self.should_fail.lock().unwrap() {
                return Err(FileImportError::ZipError("Mock error".to_string()));
            }
            Ok(self.zip_contents.lock().unwrap().clone())
        }

        fn read_file_checksum(
            &self,
            _file_path: &Path,
        ) -> Result<HashMap<Sha1Checksum, ReadFile>, FileImportError> {
            if *self.should_fail.lock().unwrap() {
                return Err(FileImportError::FileIoError("Mock error".to_string()));
            }
            Ok(self.file_checksums.lock().unwrap().clone())
        }
    }
}
