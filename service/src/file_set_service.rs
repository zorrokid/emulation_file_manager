use std::sync::Arc;

use async_trait::async_trait;
use core_types::{FileType, ImportedFile};
use database::{helper::AddFileSetParams, repository_manager::RepositoryManager};

pub struct CreateFileSetParams {
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub source: String,
    pub file_type: FileType,
    pub system_ids: Vec<i64>,
    pub files_in_file_set: Vec<ImportedFile>,
    pub create_release: bool,
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

#[derive(Debug)]
pub struct FileSetService {
    repository_manager: Arc<RepositoryManager>,
}

impl FileSetService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        FileSetService { repository_manager }
    }
}

#[async_trait]
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use core_types::{ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::file_set_service::FileSetServiceOps;

    #[async_std::test]
    async fn test_create_file_set() {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));

        let system_id = repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let file_set_service = super::FileSetService::new(Arc::clone(&repository_manager));
        let file_1_sha1: Sha1Checksum = [0u8; 20];
        let file_2_sha1: Sha1Checksum = [1u8; 20];
        let files_in_fileset: Vec<ImportedFile> = vec![
            ImportedFile {
                original_file_name: "test_file_1.rom".to_string(),
                archive_file_name: "archive_file_name".to_string(),
                sha1_checksum: file_1_sha1,
                file_size: 1024,
            },
            ImportedFile {
                original_file_name: "test_file_2.rom".to_string(),
                archive_file_name: "archive_file_name_2".to_string(),
                sha1_checksum: file_2_sha1,
                file_size: 2048,
            },
        ];
        let create_params = super::CreateFileSetParams {
            file_set_name: "Test File Set".to_string(),
            file_set_file_name: "test_file_set.zip".to_string(),
            source: "Unit Test".to_string(),
            file_type: core_types::FileType::Rom,
            system_ids: vec![system_id],
            files_in_file_set: files_in_fileset,
            create_release: true,
        };
        let result = file_set_service
            .create_file_set(create_params)
            .await
            .unwrap();
        assert!(result.file_set_id > 0);
        assert!(result.release_id.is_some());

        let release = repository_manager
            .get_release_repository()
            .get_release(result.release_id.unwrap())
            .await
            .unwrap();

        assert_eq!(release.id, result.release_id.unwrap());
        assert_eq!(release.name, "Test File Set");

        let software_titles = repository_manager
            .get_software_title_repository()
            .get_software_titles_by_release(release.id)
            .await
            .unwrap();

        assert_eq!(software_titles.len(), 1);
        assert_eq!(software_titles[0].name, "Test File Set");
    }

    #[async_std::test]
    async fn test_create_file_set_with_non_existing_system() {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));

        let file_set_service = super::FileSetService::new(Arc::clone(&repository_manager));
        let file_1_sha1: Sha1Checksum = [0u8; 20];
        let files_in_fileset: Vec<ImportedFile> = vec![ImportedFile {
            original_file_name: "test_file_1.rom".to_string(),
            archive_file_name: "archive_file_name".to_string(),
            sha1_checksum: file_1_sha1,
            file_size: 1024,
        }];
        let create_params = super::CreateFileSetParams {
            file_set_name: "Test File Set".to_string(),
            file_set_file_name: "test_file_set.zip".to_string(),
            source: "Unit Test".to_string(),
            file_type: core_types::FileType::Rom,
            system_ids: vec![123],
            files_in_file_set: files_in_fileset,
            create_release: true,
        };
        let result = file_set_service.create_file_set(create_params).await;

        assert!(result.is_err());

        // file set shouldn't exist
        let file_sets = repository_manager
            .get_file_set_repository()
            .get_all_file_sets()
            .await
            .unwrap();
        assert_eq!(file_sets.len(), 0);
    }
}
