use crate::error::EmulatorRunnerError;
#[allow(deprecated)]
use crate::run_with_emulator;
use core_types::ArgumentType;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// TODO: this should be more generic, like ExternalExecutableRunnerOps
/// Trait for emulator runner operations.
///
/// This trait abstracts emulator execution functionality to allow for different implementations,
/// including mocks for testing purposes.
#[async_trait::async_trait]
pub trait EmulatorRunnerOps: Send + Sync {
    /// Runs an emulator with the given executable, arguments, and files.
    ///
    /// # Arguments
    /// * `executable` - Emulator executable name (if on system PATH) or full path to executable
    /// * `arguments` - Arguments to pass to the emulator
    /// * `file_names` - Vector of file names to be used with emulator
    /// * `selected_file_name` - Entry point file in the set of files
    /// * `source_path` - Path where the files are located
    ///
    /// # Returns
    /// * `Ok(())` on successful execution
    /// * `Err(EmulatorRunnerError)` if execution fails
    async fn run_with_emulator(
        &self,
        executable: String,
        arguments: &[ArgumentType],
        file_names: &[String],
        selected_file_name: String,
        source_path: PathBuf,
    ) -> Result<(), EmulatorRunnerError>;
}

/// Default implementation that performs actual emulator execution.
pub struct DefaultEmulatorRunnerOps;

impl DefaultEmulatorRunnerOps {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultEmulatorRunnerOps {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl EmulatorRunnerOps for DefaultEmulatorRunnerOps {
    async fn run_with_emulator(
        &self,
        executable: String,
        arguments: &[ArgumentType],
        file_names: &[String],
        selected_file_name: String,
        source_path: PathBuf,
    ) -> Result<(), EmulatorRunnerError> {
        #[allow(deprecated)]
        run_with_emulator(
            executable,
            arguments,
            file_names,
            selected_file_name,
            source_path,
        )
        .await
    }
}

/// Represents a recorded call to an emulator runner operation.
///
/// Used by `MockEmulatorRunnerOps` to track and verify emulator calls in tests.
#[derive(Debug, Clone)]
pub struct EmulatorRunCall {
    /// Executable that was called
    pub executable: String,
    /// Arguments passed to the emulator
    pub arguments: Vec<ArgumentType>,
    /// File names in the set
    pub file_names: Vec<String>,
    /// Selected entry point file
    pub selected_file_name: String,
    /// Source path where files are located
    pub source_path: PathBuf,
}

/// Mock implementation for testing emulator runner operations.
///
/// This mock tracks all emulator run calls and can simulate failures, allowing comprehensive
/// testing without actually executing emulators.
///
/// # Examples
///
/// ```
/// use emulator_runner::ops::{EmulatorRunnerOps, MockEmulatorRunnerOps};
/// use core_types::ArgumentType;
/// use std::path::PathBuf;
///
/// #[async_std::main]
/// async fn main() {
///     // Test successful run
///     let mock = MockEmulatorRunnerOps::new();
///     let result = mock.run_with_emulator(
///         "emulator".to_string(),
///         &[ArgumentType::Flag { name: "-verbose".to_string() }],
///         &["game.rom".to_string()],
///         "game.rom".to_string(),
///         PathBuf::from("/games"),
///     ).await;
///     assert!(result.is_ok());
///
///     // Verify calls
///     assert_eq!(mock.total_calls(), 1);
///     let calls = mock.run_calls();
///     assert_eq!(calls[0].executable, "emulator");
/// }
/// ```
#[derive(Clone, Default)]
pub struct MockEmulatorRunnerOps {
    should_fail: bool,
    error_message: Option<String>,
    run_calls: Arc<Mutex<Vec<EmulatorRunCall>>>,
}

impl MockEmulatorRunnerOps {
    /// Creates a new mock that succeeds on all emulator run operations.
    ///
    /// Use this for testing happy path scenarios where emulator runs should succeed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new mock that fails on all emulator run operations with the given error message.
    ///
    /// Use this for testing error handling paths in your code.
    ///
    /// # Arguments
    /// * `error_msg` - The error message to return when emulator run operations fail
    ///
    /// # Examples
    ///
    /// ```
    /// use emulator_runner::ops::MockEmulatorRunnerOps;
    ///
    /// let mock = MockEmulatorRunnerOps::with_failure("Emulator crashed");
    /// // All emulator run operations will now fail with "Emulator crashed" error
    /// ```
    pub fn with_failure(error_msg: impl Into<String>) -> Self {
        Self {
            should_fail: true,
            error_message: Some(error_msg.into()),
            ..Default::default()
        }
    }

    /// Returns all calls made to the `run_with_emulator` method.
    pub fn run_calls(&self) -> Vec<EmulatorRunCall> {
        self.run_calls.lock().unwrap().clone()
    }

    /// Returns the total number of emulator run calls made.
    pub fn total_calls(&self) -> usize {
        self.run_calls.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl EmulatorRunnerOps for MockEmulatorRunnerOps {
    async fn run_with_emulator(
        &self,
        executable: String,
        arguments: &[ArgumentType],
        file_names: &[String],
        selected_file_name: String,
        source_path: PathBuf,
    ) -> Result<(), EmulatorRunnerError> {
        let call = EmulatorRunCall {
            executable: executable.clone(),
            arguments: arguments.to_vec(),
            file_names: file_names.to_vec(),
            selected_file_name: selected_file_name.clone(),
            source_path: source_path.clone(),
        };
        self.run_calls.lock().unwrap().push(call);

        if self.should_fail {
            return Err(EmulatorRunnerError::IoError(
                self.error_message
                    .clone()
                    .unwrap_or_else(|| "Mock emulator run failed".to_string()),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_types::ArgumentType;
    use std::path::PathBuf;

    #[async_std::test]
    async fn test_mock_emulator_runner_ops_success() {
        let mock = MockEmulatorRunnerOps::new();

        let result = mock
            .run_with_emulator(
                "emulator".to_string(),
                &[ArgumentType::Flag {
                    name: "-verbose".to_string(),
                }],
                &["game.rom".to_string()],
                "game.rom".to_string(),
                PathBuf::from("/games"),
            )
            .await;

        assert!(result.is_ok());

        // Verify the call was tracked
        assert_eq!(mock.total_calls(), 1);
        let calls = mock.run_calls();
        assert_eq!(calls.len(), 1);

        let call = &calls[0];
        assert_eq!(call.executable, "emulator");
        assert_eq!(call.file_names, vec!["game.rom"]);
        assert_eq!(call.selected_file_name, "game.rom");
        assert_eq!(call.source_path, PathBuf::from("/games"));
    }

    #[async_std::test]
    async fn test_mock_emulator_runner_ops_failure() {
        let mock = MockEmulatorRunnerOps::with_failure("Simulated emulator crash");

        let result = mock
            .run_with_emulator(
                "emulator".to_string(),
                &[],
                &["game.rom".to_string()],
                "game.rom".to_string(),
                PathBuf::from("/games"),
            )
            .await;

        assert!(result.is_err());

        // Verify the call was tracked even though it failed
        assert_eq!(mock.total_calls(), 1);

        match result {
            Err(EmulatorRunnerError::IoError(msg)) => {
                assert_eq!(msg, "Simulated emulator crash");
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[async_std::test]
    async fn test_mock_tracks_multiple_calls() {
        let mock = MockEmulatorRunnerOps::new();

        mock.run_with_emulator(
            "emulator1".to_string(),
            &[],
            &["game1.rom".to_string()],
            "game1.rom".to_string(),
            PathBuf::from("/games"),
        )
        .await
        .unwrap();

        mock.run_with_emulator(
            "emulator2".to_string(),
            &[ArgumentType::FlagWithValue {
                name: "-config".to_string(),
                value: "config.ini".to_string(),
            }],
            &["game2.rom".to_string()],
            "game2.rom".to_string(),
            PathBuf::from("/other"),
        )
        .await
        .unwrap();

        assert_eq!(mock.total_calls(), 2);
        let calls = mock.run_calls();
        assert_eq!(calls[0].executable, "emulator1");
        assert_eq!(calls[1].executable, "emulator2");
    }
}
