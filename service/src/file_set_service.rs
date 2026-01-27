use std::sync::Arc;

use core_types::{FileType, ImportedFile};
use database::repository_manager::RepositoryManager;

pub struct CreateFileSetParams {
    file_set_name: String,
    file_set_file_name: String,
    source: String,
    file_type: FileType,
    system_ids: Vec<i64>,
    files_in_file_set: Vec<ImportedFile>,
    create_release: bool,
}

#[derive(Debug)]
pub struct CreateFileSetResult {
    pub file_set_id: i64,
    pub release_id: Option<i64>,
    pub software_title_id: Option<i64>,
}

#[derive(Debug)]
pub enum FileSetServiceError {
    DatabaseError(String),
}

pub trait FileSetServiceOps {
    fn create_file_set(
        &self,
        file_set_params: CreateFileSetParams,
    ) -> Result<CreateFileSetResult, FileSetServiceError>;
}

#[derive(Debug)]
pub struct FileSetService {
    repository_manager: Arc<RepositoryManager>,
}

impl FileSetService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        FileSetService { repository_manager }
    }
}

impl FileSetServiceOps for FileSetService {
    fn create_file_set(
        &self,
        file_set_params: CreateFileSetParams,
    ) -> Result<CreateFileSetResult, FileSetServiceError> {
        // Implementation to create file set, software title, and release in the database
        // using self.repository_manager

        // Placeholder implementation
        Ok(CreateFileSetResult {
            file_set_id: 1,
            release_id: if file_set_params.create_release {
                Some(1)
            } else {
                None
            },
            software_title_id: if file_set_params.create_release {
                Some(1)
            } else {
                None
            },
        })
    }
}
