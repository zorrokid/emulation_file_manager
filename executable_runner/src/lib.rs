use async_process::Command;
use core_types::ArgumentType;
use std::path::{Path, PathBuf};

use error::ExecutableRunnerError;

pub mod error;
pub mod ops;

/// Asynchronous function to run an the given executable, arguments, and file names.
/// It takes the selected file name and source path to locate the file.
///
/// # arguments
/// * `executable`: executable name (if it's found on system path) or the full path to the executable.
/// * `arguments`: The arguments to pass to the executable.
/// * `file_names`: A vector of file names to be used with executable.
/// * `selected_file_name`: The name of the entry point file of the set of file_names to be executed.
/// * `source_path`: The path where the files are located.
///
/// # returns
/// * `Result<(), ExecutableRunnerError>`: Returns Ok if the executable runs successfully, or an error if it fails.
///
/// # errors
/// * `ExecutableRunnerError::NoFileSelected`: If no file is selected.
/// * `ExecutableRunnerError::FileNotFound`: If the selected file is not found.
/// * `ExecutableRunnerError::IoError`: If there is an IO error while running the executable.
#[deprecated(note = "Use ExecutableRunnerOps trait instead")]
pub async fn run_executable(
    executable: String,
    arguments: &[ArgumentType],
    file_names: &[String],      // list of files selected for running
    selected_file_name: String, // entry point file in possible set of files
    source_path: PathBuf,       // where to find files
) -> Result<(), ExecutableRunnerError> {
    if file_names.is_empty() {
        return Err(ExecutableRunnerError::NoFileSelected);
    }
    let file_path = Path::new(&source_path).join(&selected_file_name);

    tracing::debug!("Emulator executable: {}", executable);
    tracing::debug!("Emulator arguments: {:?}", arguments);
    tracing::debug!("File to run: {}", file_path.display());

    if !file_path.exists() {
        return Err(ExecutableRunnerError::FileNotFound);
    }

    let mut command = Command::new(&executable);

    if arguments.is_empty() {
        tracing::debug!("No arguments provided, running with file path as only argument.");
        command.arg(&file_path).current_dir(&source_path);
    } else {
        tracing::debug!("Preparing to run executable with arguments {:?}", arguments);

        let mut args = Vec::new();

        arguments.iter().for_each(|arg| match arg {
            ArgumentType::Flag { name } => {
                args.push(name.clone());
            }
            ArgumentType::FlagWithValue { name, value } => {
                args.extend_from_slice(&[name.clone(), value.clone()]);
            }
            ArgumentType::FlagEqualsValue { name, value } => {
                // TODO: check if this is working as expected
                args.push(format!("{}={}", name, value));
            }
        });

        command
            .args(&args)
            .arg(&file_path)
            .current_dir(&source_path);
    }

    tracing::debug!("Command to execute: {:?}", command);

    let status = command.status().await.map_err(|e| {
        ExecutableRunnerError::IoError(format!("Failed to get status of executable: {}", e))
    })?;

    if !status.success() {
        return Err(ExecutableRunnerError::IoError(format!(
            "Emulator failed with status: {}",
            status
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[async_std::test]
    async fn test_run_executable() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let file_name = "test.d64";
        let file_path = output_path.join(file_name);
        std::fs::write(&file_path, "test data").unwrap();
        let executable = "echo".to_string();
        let arguments = ArgumentType::Flag {
            name: "hello".into(),
        };
        let file_names = vec![file_name.to_string()];
        let selected_file_name = file_name.to_string();
        let source_path = output_path.to_path_buf();
        let result = run_executable(
            executable,
            &[arguments],
            &file_names,
            selected_file_name,
            source_path,
        )
        .await;
        assert!(result.is_ok(), "Executable run failed: {:?}", result);
    }
}
