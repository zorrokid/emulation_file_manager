use core_types::{ImportedFile, Sha1Checksum};
use std::collections::HashMap;

use crate::{FileImportError, FileImportModel};

/// Trait for file import operations to enable testing
///
/// This trait abstracts file reading and checksum calculation operations,
/// allowing them to be mocked in tests.
pub trait FileImportOps: Send + Sync {
    /// Import files based on the provided model
    ///
    /// Reads files from disk, optionally extracts from ZIP, and writes them to the output directory.
    /// Returns a map of SHA1 checksums to imported file information.
    fn import(
        &self,
        file_import_model: &FileImportModel,
    ) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError>;
}

/// Standard implementation using actual file system operations
#[derive(Debug, Clone)]
pub struct StdFileImportOps;

impl FileImportOps for StdFileImportOps {
    fn import(
        &self,
        file_import_model: &FileImportModel,
    ) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
        crate::import(file_import_model)
    }
}

pub mod mock {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Mock implementation for testing file import operations
    #[derive(Clone, Default)]
    pub struct MockFileImportOps {
        imported_files: Arc<Mutex<HashMap<Sha1Checksum, ImportedFile>>>,
        should_fail: Arc<Mutex<bool>>,
    }

    impl MockFileImportOps {
        pub fn new() -> Self {
            Self::default()
        }

        /// Add an imported file to be returned by import
        pub fn add_imported_file(&self, checksum: Sha1Checksum, imported_file: ImportedFile) {
            self.imported_files
                .lock()
                .unwrap()
                .insert(checksum, imported_file);
        }

        /// Make all operations fail with an error
        pub fn set_should_fail(&self, should_fail: bool) {
            *self.should_fail.lock().unwrap() = should_fail;
        }
    }

    impl FileImportOps for MockFileImportOps {
        fn import(
            &self,
            file_import_model: &FileImportModel,
        ) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
            println!("Mock import called with model: {:?}", file_import_model);
            if *self.should_fail.lock().unwrap() {
                return Err(FileImportError::FileIoError(
                    "Mock import error".to_string(),
                ));
            }
            Ok(self.imported_files.lock().unwrap().clone())
        }
    }
}
