use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmulatorRunnerError {
    #[error("IO error: {0}")]
    IoError(String),
    #[error("No file selected")]
    NoFileSelected,
    #[error("File not found")]
    FileNotFound,
}
