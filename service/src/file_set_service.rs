use std::sync::Arc;

use core_types::{FileType, ImportedFile};
use database::{helper::AddFileSetParams, repository_manager::RepositoryManager};

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
}

#[derive(Debug)]
pub enum FileSetServiceError {
    DatabaseError(String),
}

pub trait FileSetServiceOps {
    async fn create_file_set(
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
    async fn create_file_set(
        &self,
        file_set_params: CreateFileSetParams,
    ) -> Result<CreateFileSetResult, FileSetServiceError> {
        let mut transaction = self
            .repository_manager
            .begin_transaction()
            .await
            .map_err(|e| FileSetServiceError::DatabaseError(format!("{:?}", e)))?;

        let add_file_set_params = AddFileSetParams {
            file_set_name: &file_set_params.file_set_name,
            file_set_file_name: &file_set_params.file_set_file_name,
            source: &file_set_params.source,
            file_type: &file_set_params.file_type,
            system_ids: &file_set_params.system_ids,
            files_in_fileset: &file_set_params.files_in_file_set,
        };

        let file_set_id = self
            .repository_manager
            .get_file_set_repository()
            .add_file_set_with_tx(&mut transaction, add_file_set_params)
            .await
            .map_err(|e| FileSetServiceError::DatabaseError(format!("{:?}", e)))?;

        let release_id = if file_set_params.create_release {
            let software_title_id = self
                .repository_manager
                .get_software_title_repository()
                .add_software_title_with_tx(&mut transaction, &file_set_params.file_set_name, None)
                .await
                .map_err(|e| FileSetServiceError::DatabaseError(format!("{:?}", e)))?;

            Some(
                self.repository_manager
                    .get_release_repository()
                    .add_release_full_with_tx(
                        &mut transaction,
                        &file_set_params.file_set_name,
                        &[software_title_id],
                        &[file_set_id],
                        &file_set_params.system_ids,
                    )
                    .await
                    .map_err(|e| FileSetServiceError::DatabaseError(format!("{:?}", e)))?,
            )
        } else {
            None
        };

        transaction
            .commit()
            .await
            .map_err(|e| FileSetServiceError::DatabaseError(format!("{:?}", e)))?;

        Ok(CreateFileSetResult {
            file_set_id,
            release_id,
        })
    }
}
