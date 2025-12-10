use std::{collections::HashMap, sync::Arc};

use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    file_set_deletion::{context::DeletionContext, model::FileDeletionResult},
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

pub struct FileSetDeletionService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    fs_ops: Arc<dyn FileSystemOps>,
}

impl std::fmt::Debug for FileSetDeletionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileSetDeletionService")
            .field("repository_manager", &"Arc<RepositoryManager>")
            .field("settings", &self.settings)
            .field("fs_ops", &"Arc<dyn FileSystemOps>")
            .finish()
    }
}

impl FileSetDeletionService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self::new_with_fs_ops(repository_manager, settings, Arc::new(StdFileSystemOps))
    }

    pub fn new_with_fs_ops(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            fs_ops,
        }
    }

    pub async fn delete_file_set(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<FileDeletionResult>, Error> {
        tracing::info!("Starting deletion for file set ID {}", file_set_id);
        let mut context = DeletionContext {
            file_set_id,
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
            fs_ops: self.fs_ops.clone(),
            deletion_results: HashMap::new(),
        };

        let pipeline = Pipeline::<DeletionContext>::new();
        pipeline.execute(&mut context).await?;

        tracing::info!("Completed deletion for file set ID {}", file_set_id);
        Ok(context.deletion_results.values().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use core_types::{FileSyncStatus, FileType, ImportedFile, Sha1Checksum};
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

        let file_info_id = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap()[0]
            .id;

        // add sync log entry to mark that file has been synced to cloud
        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadCompleted,
                "",
                "rom/game.zst",
            )
            .await
            .unwrap();

        let service =
            FileSetDeletionService::new_with_fs_ops(repo_manager, settings, mock_fs.clone());

        let result = service.delete_file_set(file_set_id).await;
        assert!(result.is_ok());
        let file_deletion_result = result.unwrap();
        assert_eq!(file_deletion_result.len(), 1);
        let deletion_info = &file_deletion_result[0];
        assert!(deletion_info.file_deletion_success.unwrap());
        assert!(deletion_info.db_deletion_success.unwrap());
        assert!(deletion_info.cloud_delete_marked_successfully.unwrap());
    }
}
