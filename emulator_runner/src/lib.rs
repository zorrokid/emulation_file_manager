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

    println!(
        "Running {} with emulator {}",
        file_path.to_string_lossy(),
        executable
    );

    let mut command = Command::new(&executable);

    command.arg(&file_path).current_dir(source_path);

    if !arguments.is_empty() {
        // TODO: should use command.args() instead and emulator arguments should be split into separate strings
        command.arg(&arguments);
    }

    let status = command.status().await.map_err(|e| {
        EmulatorRunnerError::IoError(format!("Failed to get status of emulator: {}", e))
    })?;
    println!("Emulator exited with status: {}", status);
    if !status.success() {
        eprintln!("Emulator failed with status: {}", status);
    }
    println!("Finished running with emulator");

    Ok(())
}
