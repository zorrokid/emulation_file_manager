use std::{collections::HashMap, sync::Arc};

use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    file_set_deletion::context::DeletionContext,
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

#[derive(Debug)]
pub struct FileSetDeletionService<F: FileSystemOps = StdFileSystemOps> {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    fs_ops: Arc<F>,
}

impl FileSetDeletionService<StdFileSystemOps> {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self::new_with_fs_ops(repository_manager, settings, Arc::new(StdFileSystemOps))
    }
}

impl<F: FileSystemOps> FileSetDeletionService<F> {
    pub fn new_with_fs_ops(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        fs_ops: Arc<F>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            fs_ops,
        }
    }

    pub async fn delete_file_set(&self, file_set_id: i64) -> Result<(), Error> {
        let mut context = DeletionContext {
            file_set_id,
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
            fs_ops: self.fs_ops.clone(),
            deletion_results: HashMap::new(),
        };

        let pipeline = Pipeline::<DeletionContext<F>>::new();
        pipeline.execute(&mut context).await?;

        let successful_deletions = context
            .deletion_results
            .values()
            .filter(|r| r.file_deletion_success && r.was_deleted_from_db)
            .count();
        let failed_deletions = context
            .deletion_results
            .values()
            .filter(|r| !r.file_deletion_success || !r.was_deleted_from_db)
            .count();

        eprintln!(
            "Deletion complete: {} successful, {} failed",
            successful_deletions, failed_deletions
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::setup_test_db;

    use super::*;
    use crate::file_system_ops::mock::MockFileSystemOps;

    #[async_std::test]
    async fn test_delete_file_set() {
        let test_db_pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(test_db_pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });

        let system_id = repo_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let file1 = ImportedFile {
            original_file_name: "file1.zst".to_string(),
            archive_file_name: "file1.zst".to_string(),
            sha1_checksum: Sha1Checksum::from([0; 20]),
            file_size: 1234,
        };

        let mock_fs = Arc::new(MockFileSystemOps::new());
        let file_path = settings.get_file_path(&FileType::Rom, &file1.archive_file_name);
        mock_fs.add_file(file_path.to_string_lossy().as_ref());

        let file_set_id = repo_manager
            .get_file_set_repository()
            .add_file_set(
                "test_set",
                "file name",
                &FileType::Rom,
                "",
                &[file1],
                &[system_id],
            )
            .await
            .unwrap();

        let service =
            FileSetDeletionService::new_with_fs_ops(repo_manager, settings, mock_fs.clone());

        let result = service.delete_file_set(file_set_id).await;
        assert!(result.is_ok());
    }
}
