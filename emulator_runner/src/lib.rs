use async_process::Command;
use std::path::{Path, PathBuf};

use error::EmulatorRunnerError;

pub mod error;

pub async fn run_with_emulator(
    executable: String,
    arguments: String,
    file_names: Vec<String>,    // list of files selected for running
    selected_file_name: String, // entry point file in possible set of files
    source_path: PathBuf,       // where to find files
) -> Result<(), EmulatorRunnerError> {
    if file_names.is_empty() {
        return Err(EmulatorRunnerError::NoFileSelected);
    }
    let file_path = Path::new(&source_path).join(&selected_file_name);
    if !file_path.exists() {
        return Err(EmulatorRunnerError::FileNotFound);
    }

    let mut command = Command::new(&executable);

    command.arg(&file_path).current_dir(source_path);

    if !arguments.is_empty() {
        // TODO: should use command.args() instead and emulator arguments should be split into separate strings
        command.arg(&arguments);
    }

    let status = command.status().await.map_err(|e| {
        EmulatorRunnerError::IoError(format!("Failed to get status of emulator: {}", e))
    })?;

    if !status.success() {
        return Err(EmulatorRunnerError::IoError(format!(
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
    async fn test_run_with_emulator() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let file_name = "test.d64";
        let file_path = output_path.join(file_name);
        std::fs::write(&file_path, "test data").unwrap();
        let executable = "echo".to_string();
        let arguments = "Hello, world!".to_string();
        let file_names = vec![file_name.to_string()];
        let selected_file_name = file_name.to_string();
        let source_path = output_path.to_path_buf();
        let result = run_with_emulator(
            executable,
            arguments,
            file_names,
            selected_file_name,
            source_path,
        )
        .await;
        assert!(result.is_ok(), "Emulator run failed: {:?}", result);
    }
}
