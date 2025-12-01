use crate::error::ExecutableRunnerError;
#[allow(deprecated)]
use crate::run_executable;
use core_types::ArgumentType;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// TODO: this should be more generic, like ExternalExecutableRunnerOps
/// Trait for executable runner operations.
///
/// This trait abstracts executable execution functionality to allow for different implementations,
/// including mocks for testing purposes.
#[async_trait::async_trait]
pub trait ExecutableRunnerOps: Send + Sync {
    /// Runs the given executable, arguments, and files.
    ///
    /// # Arguments
    /// * `executable` - Executable name (if on system PATH) or full path to executable
    /// * `arguments` - Arguments to pass to the executable  
    /// * `file_names` - Vector of file names to be used with executable
    /// * `selected_file_name` - Entry point file in the set of files
    /// * `source_path` - Path where the files are located
    ///
    /// # Returns
    /// * `Ok(())` on successful execution
    /// * `Err(ExecutableRunnerError)` if execution fails
    async fn run_executable(
        &self,
        executable: String,
        arguments: &[ArgumentType],
        file_names: &[String],
        selected_file_name: String,
        source_path: PathBuf,
    ) -> Result<(), ExecutableRunnerError>;
}

/// Default implementation that performs actual executable execution.
pub struct DefaultExecutableRunnerOps;

impl DefaultExecutableRunnerOps {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultExecutableRunnerOps {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ExecutableRunnerOps for DefaultExecutableRunnerOps {
    async fn run_executable(
        &self,
        executable: String,
        arguments: &[ArgumentType],
        file_names: &[String],
        selected_file_name: String,
        source_path: PathBuf,
    ) -> Result<(), ExecutableRunnerError> {
        #[allow(deprecated)]
        run_executable(
            executable,
            arguments,
            file_names,
            selected_file_name,
            source_path,
        )
        .await
    }
}

/// Represents a recorded call to an executable runner operation.
///
/// Used by `MockExecutableRunnerOps` to track and verify executable calls in tests.
#[derive(Debug, Clone)]
pub struct ExecutableRunCall {
    /// Executable that was called
    pub executable: String,
    /// Arguments passed to the executable
    pub arguments: Vec<ArgumentType>,
    /// File names in the set
    pub file_names: Vec<String>,
    /// Selected entry point file
    pub selected_file_name: String,
    /// Source path where files are located
    pub source_path: PathBuf,
}

/// Mock implementation for testing executable runner operations.
///
/// This mock tracks all executable run calls and can simulate failures, allowing comprehensive
/// testing without actually executing executables.
///
/// # Examples
///
/// ```
/// use executable_runner::ops::{ExecutableRunnerOps, MockExecutableRunnerOps};
/// use core_types::ArgumentType;
/// use std::path::PathBuf;
///
/// #[async_std::main]
/// async fn main() {
///     // Test successful run
///     let mock = MockExecutableRunnerOps::new();
///     let result = mock.run_executable(
///         "executable".to_string(),
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
///     assert_eq!(calls[0].executable, "executable");
/// }
/// ```
#[derive(Clone, Default)]
pub struct MockExecutableRunnerOps {
    should_fail: bool,
    error_message: Option<String>,
    run_calls: Arc<Mutex<Vec<ExecutableRunCall>>>,
}

impl MockExecutableRunnerOps {
    /// Creates a new mock that succeeds on all executable run operations.
    ///
    /// Use this for testing happy path scenarios where executable runs should succeed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new mock that fails on all executable run operations with the given error message.
    ///
    /// Use this for testing error handling paths in your code.
    ///
    /// # Arguments
    /// * `error_msg` - The error message to return when executable run operations fail
    ///
    /// # Examples
    ///
    /// ```
    /// use executable_runner::ops::MockExecutableRunnerOps;
    ///
    /// let mock = MockExecutableRunnerOps::with_failure("Executable crashed");
    /// // All executable run operations will now fail with "Executable crashed" error
    /// ```
    pub fn with_failure(error_msg: impl Into<String>) -> Self {
        Self {
            should_fail: true,
            error_message: Some(error_msg.into()),
            ..Default::default()
        }
    }

    /// Returns all calls made to the `run_executable` method.
    pub fn run_calls(&self) -> Vec<ExecutableRunCall> {
        self.run_calls.lock().unwrap().clone()
    }

    /// Returns the total number of executable run calls made.
    pub fn total_calls(&self) -> usize {
        self.run_calls.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl ExecutableRunnerOps for MockExecutableRunnerOps {
    async fn run_executable(
        &self,
        executable: String,
        arguments: &[ArgumentType],
        file_names: &[String],
        selected_file_name: String,
        source_path: PathBuf,
    ) -> Result<(), ExecutableRunnerError> {
        let call = ExecutableRunCall {
            executable: executable.clone(),
            arguments: arguments.to_vec(),
            file_names: file_names.to_vec(),
            selected_file_name: selected_file_name.clone(),
            source_path: source_path.clone(),
        };
        self.run_calls.lock().unwrap().push(call);

        if self.should_fail {
            return Err(ExecutableRunnerError::IoError(
                self.error_message
                    .clone()
                    .unwrap_or_else(|| "Mock executable run failed".to_string()),
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
    async fn test_mock_executable_runner_ops_success() {
        let mock = MockExecutableRunnerOps::new();

        let result = mock
            .run_executable(
                "executable".to_string(),
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
        assert_eq!(call.executable, "executable");
        assert_eq!(call.file_names, vec!["game.rom"]);
        assert_eq!(call.selected_file_name, "game.rom");
        assert_eq!(call.source_path, PathBuf::from("/games"));
    }

    #[async_std::test]
    async fn test_mock_executable_runner_ops_failure() {
        let mock = MockExecutableRunnerOps::with_failure("Simulated executable crash");

        let result = mock
            .run_executable(
                "executable".to_string(),
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
            Err(ExecutableRunnerError::IoError(msg)) => {
                assert_eq!(msg, "Simulated executable crash");
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[async_std::test]
    async fn test_mock_tracks_multiple_calls() {
        let mock = MockExecutableRunnerOps::new();

        mock.run_executable(
            "executable1".to_string(),
            &[],
            &["game1.rom".to_string()],
            "game1.rom".to_string(),
            PathBuf::from("/games"),
        )
        .await
        .unwrap();

        mock.run_executable(
            "executable2".to_string(),
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
        assert_eq!(calls[0].executable, "executable1");
        assert_eq!(calls[1].executable, "executable2");
    }
}
