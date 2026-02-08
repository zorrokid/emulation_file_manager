pub mod file_set_service;
pub mod mock_file_set_service;

use async_trait::async_trait;
use core_types::{FileType, ImportedFile};

use crate::file_import::model::CreateReleaseParams;

pub struct CreateFileSetParams {
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub source: String,
    pub file_type: FileType,
    pub system_ids: Vec<i64>,
    pub files_in_file_set: Vec<ImportedFile>,
    pub create_release: Option<CreateReleaseParams>,
    pub dat_file_id: Option<i64>,
}

#[derive(Debug)]
pub struct CreateFileSetResult {
    pub file_set_id: i64,
    pub release_id: Option<i64>,
}

#[derive(Debug)]
pub enum FileSetServiceError {
    DatabaseError(String),
}

impl std::fmt::Display for FileSetServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileSetServiceError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
        }
    }
}

#[async_trait]
pub trait FileSetServiceOps: Send + Sync {
    async fn create_file_set(
        &self,
        file_set_params: CreateFileSetParams,
    ) -> Result<CreateFileSetResult, FileSetServiceError>;
}
