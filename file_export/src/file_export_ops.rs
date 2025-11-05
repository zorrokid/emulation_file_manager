use crate::{export_files, export_files_zipped, FileExportError, FileSetExportModel};
use std::sync::{Arc, Mutex};

/// Trait for file export operations.
///
/// This trait abstracts file export functionality to allow for different implementations,
/// including mocks for testing purposes.
pub trait FileExportOps: Send + Sync {
    /// Exports files from zstd archive to individual decompressed files.
    ///
    /// # Arguments
    /// * `export_model` - Configuration model containing source paths, output mappings, and checksums
    ///
    /// # Returns
    /// * `Ok(())` on successful export
    /// * `Err(FileExportError)` if export fails
    fn export(&self, export_model: &FileSetExportModel) -> Result<(), FileExportError>;

    /// Exports files from zstd archive to a single zip file.
    ///
    /// # Arguments
    /// * `export_model` - Configuration model containing source paths, output mappings, and checksums
    ///
    /// # Returns
    /// * `Ok(())` on successful export
    /// * `Err(FileExportError)` if export fails
    fn export_zipped(&self, export_model: &FileSetExportModel) -> Result<(), FileExportError>;
}

/// Default implementation that performs actual file export operations.
pub struct DefaultFileExportOps;

impl FileExportOps for DefaultFileExportOps {
    fn export(&self, export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        export_files(export_model)
    }

    fn export_zipped(&self, export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        export_files_zipped(export_model)
    }
}

/// Represents a recorded call to an export operation.
///
/// Used by `MockFileExportOps` to track and verify export calls in tests.
#[derive(Debug, Clone)]
pub struct ExportCall {
    /// Output file names that were requested in the export
    pub output_file_names: Vec<String>,
    /// Source file path used for the export
    pub source_file_path: String,
    /// Whether files should be extracted individually (true) or zipped (false)
    pub extract_files: bool,
}

/// Shared state for tracking mock calls.
///
/// This allows tests to inspect what calls were made to the mock without
/// needing to downcast the trait object.
#[derive(Default)]
pub struct MockState {
    export_calls: Arc<Mutex<Vec<ExportCall>>>,
    export_zipped_calls: Arc<Mutex<Vec<ExportCall>>>,
}

impl MockState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all calls made to the `export` method.
    pub fn export_calls(&self) -> Vec<ExportCall> {
        self.export_calls.lock().unwrap().clone()
    }

    /// Returns all calls made to the `export_zipped` method.
    pub fn export_zipped_calls(&self) -> Vec<ExportCall> {
        self.export_zipped_calls.lock().unwrap().clone()
    }

    /// Returns the total number of export calls made.
    pub fn total_calls(&self) -> usize {
        self.export_calls.lock().unwrap().len() + self.export_zipped_calls.lock().unwrap().len()
    }

    fn record_export(&self, call: ExportCall) {
        self.export_calls.lock().unwrap().push(call);
    }

    fn record_export_zipped(&self, call: ExportCall) {
        self.export_zipped_calls.lock().unwrap().push(call);
    }
}

/// Mock implementation for testing file export operations.
///
/// This mock tracks all export calls and can simulate failures, allowing comprehensive
/// testing without performing actual file I/O operations.
///
/// # Examples
///
/// no_run
/// use file_export::file_export_ops::{FileExportOps, MockFileExportOps};
/// use file_export::{FileSetExportModel, OutputFile};
/// use std::collections::HashMap;
/// use std::path::PathBuf;
/// use core_types::Sha1Checksum;
///
/// // Test successful export
/// let mock = MockFileExportOps::new();
/// let mut output_mapping = HashMap::new();
/// output_mapping.insert(
///     "archive_file".to_string(),
///     OutputFile {
///         output_file_name: "output.rom".to_string(),
///         checksum: Sha1Checksum::from([1; 20]),
///     },
/// );
///
/// let export_model = FileSetExportModel {
///     output_mapping,
///     source_file_path: PathBuf::from("/source"),
///     extract_files: false,
///     exported_zip_file_name: "test.zip".to_string(),
///     output_dir: PathBuf::from("/output"),
/// };
///
/// let result = mock.export_zipped(&export_model);
/// assert!(result.is_ok());
///
/// // Verify via shared state
/// assert_eq!(state.total_calls(), 1);
/// assert_eq!(state.export_zipped_calls().len(), 1);
/// ```
pub struct MockFileExportOps {
    should_fail: bool,
    error_message: Option<String>,
    state: Arc<MockState>,
}

impl MockFileExportOps {
    /// Creates a new mock that succeeds on all export operations.
    ///
    /// Use this for testing happy path scenarios where exports should succeed.
    pub fn new() -> Self {
        Self::new_with_state(Arc::new(MockState::new()))
    }

    /// Creates a new mock with shared state for inspection.
    ///
    /// This allows tests to verify calls made to the mock.
    ///
    /// # Arguments
    /// * `state` - Shared state for tracking calls
    ///
    /// # Examples
    ///
    /// ```
    /// use file_export::file_export_ops::{MockFileExportOps, MockState};
    /// use std::sync::Arc;
    ///
    /// let state = Arc::new(MockState::new());
    /// let mock = MockFileExportOps::new_with_state(state.clone());
    /// // ... use mock in tests ...
    /// // Verify via state
    /// assert_eq!(state.total_calls(), 0);
    /// ```
    pub fn new_with_state(state: Arc<MockState>) -> Self {
        Self {
            should_fail: false,
            error_message: None,
            state,
        }
    }

    /// Creates a new mock that fails on all export operations with the given error message.
    ///
    /// Use this for testing error handling paths in your code.
    ///
    /// # Arguments
    /// * `error_msg` - The error message to return when export operations fail
    ///
    /// # Examples
    ///
    /// ```
    /// use file_export::file_export_ops::MockFileExportOps;
    ///
    /// let mock = MockFileExportOps::with_failure("Disk full");
    /// // All export operations will now fail with "Disk full" error
    /// ```
    pub fn with_failure(error_msg: impl Into<String>) -> Self {
        Self::with_failure_and_state(error_msg, Arc::new(MockState::new()))
    }

    /// Creates a new mock with failure and shared state.
    pub fn with_failure_and_state(error_msg: impl Into<String>, state: Arc<MockState>) -> Self {
        Self {
            should_fail: true,
            error_message: Some(error_msg.into()),
            state,
        }
    }
}

impl Default for MockFileExportOps {
    fn default() -> Self {
        Self::new()
    }
}

impl FileExportOps for MockFileExportOps {
    fn export(&self, export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        let call = ExportCall {
            output_file_names: export_model
                .output_mapping
                .values()
                .map(|f| f.output_file_name.clone())
                .collect(),
            source_file_path: export_model.source_file_path.to_string_lossy().to_string(),
            extract_files: export_model.extract_files,
        };
        self.state.record_export(call);

        if self.should_fail {
            return Err(FileExportError::FileIoError(
                self.error_message
                    .clone()
                    .unwrap_or_else(|| "Mock export failed".to_string()),
            ));
        }
        Ok(())
    }

    fn export_zipped(&self, export_model: &FileSetExportModel) -> Result<(), FileExportError> {
        let call = ExportCall {
            output_file_names: export_model
                .output_mapping
                .values()
                .map(|f| f.output_file_name.clone())
                .collect(),
            source_file_path: export_model.source_file_path.to_string_lossy().to_string(),
            extract_files: export_model.extract_files,
        };
        self.state.record_export_zipped(call);

        if self.should_fail {
            return Err(FileExportError::ZipError(
                self.error_message
                    .clone()
                    .unwrap_or_else(|| "Mock export zipped failed".to_string()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use core_types::Sha1Checksum;

    use crate::{
        file_export_ops::{FileExportOps, MockFileExportOps, MockState},
        FileExportError, FileSetExportModel, OutputFile,
    };

    #[test]
    fn test_mock_file_export_ops_success() {
        let state = std::sync::Arc::new(MockState::new());
        let mock = MockFileExportOps::new_with_state(state.clone());

        let mut output_mapping = HashMap::new();
        output_mapping.insert(
            "archive_file".to_string(),
            OutputFile {
                output_file_name: "output_file.rom".to_string(),
                checksum: Sha1Checksum::from([1; 20]),
            },
        );

        let export_model = FileSetExportModel {
            output_mapping,
            source_file_path: PathBuf::from("/source"),
            extract_files: false,
            exported_zip_file_name: "test.zip".to_string(),
            output_dir: PathBuf::from("/output"),
        };

        // Test successful export
        let result = mock.export_zipped(&export_model);
        assert!(result.is_ok());

        // Verify the call was tracked via shared state
        assert_eq!(state.total_calls(), 1);
        assert_eq!(state.export_zipped_calls().len(), 1);

        let call = &state.export_zipped_calls()[0];
        assert_eq!(call.output_file_names, vec!["output_file.rom"]);
        assert_eq!(call.source_file_path, "/source");
        assert!(!call.extract_files);
    }

    #[test]
    fn test_mock_file_export_ops_failure() {
        let state = std::sync::Arc::new(MockState::new());
        let mock =
            MockFileExportOps::with_failure_and_state("Simulated disk full error", state.clone());

        let mut output_mapping = HashMap::new();
        output_mapping.insert(
            "archive_file".to_string(),
            OutputFile {
                output_file_name: "output_file.rom".to_string(),
                checksum: Sha1Checksum::from([1; 20]),
            },
        );

        let export_model = FileSetExportModel {
            output_mapping,
            source_file_path: PathBuf::from("/source"),
            extract_files: true,
            exported_zip_file_name: "test.zip".to_string(),
            output_dir: PathBuf::from("/output"),
        };

        // Test failed export
        let result = mock.export(&export_model);
        assert!(result.is_err());

        // Verify the call was tracked even though it failed
        assert_eq!(state.total_calls(), 1);
        assert_eq!(state.export_calls().len(), 1);

        match result {
            Err(FileExportError::FileIoError(msg)) => {
                assert_eq!(msg, "Simulated disk full error");
            }
            _ => panic!("Expected FileIoError"),
        }
    }
}
