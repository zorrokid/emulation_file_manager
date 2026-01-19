use std::fmt::{Display, Formatter, Result};

use file_export::FileExportError;

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    DbError(String),
    DeserializationError(String),
    ExportError(String),
    IoError(String),
    CloudSyncError(String),
    SettingsError(String),
    DownloadError(String),
    FileImportError(String),
    OperationCancelled,
    InvalidInput(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Error::DbError(message) => write!(f, "Database error: {}", message),
            Error::DeserializationError(message) => write!(f, "Deserialization error: {}", message),
            Error::ExportError(message) => write!(f, "Export error: {}", message),
            Error::IoError(message) => write!(f, "IO error: {}", message),
            Error::CloudSyncError(message) => write!(f, "Cloud sync error: {}", message),
            Error::SettingsError(message) => write!(f, "Settings error: {}", message),
            Error::DownloadError(message) => write!(f, "Download error: {}", message),
            Error::FileImportError(message) => write!(f, "File import error: {}", message),
            Error::OperationCancelled => write!(f, "Operation was cancelled"),
            Error::InvalidInput(message) => write!(f, "Invalid input: {}", message),
        }
    }
}

impl From<database::database_error::DatabaseError> for Error {
    fn from(err: database::database_error::DatabaseError) -> Self {
        Error::DbError(err.to_string())
    }
}

impl From<cloud_storage::CloudStorageError> for Error {
    fn from(err: cloud_storage::CloudStorageError) -> Self {
        Error::CloudSyncError(err.to_string())
    }
}

impl From<FileExportError> for Error {
    fn from(err: FileExportError) -> Self {
        Error::ExportError(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err.to_string())
    }
}
