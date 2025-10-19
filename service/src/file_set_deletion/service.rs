use std::{collections::HashMap, sync::Arc};

use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    file_set_deletion::executor::DeletionPipeline,
    file_system_ops::{FileSystemOps, StdFileSystemOps},
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
        let mut context = crate::file_set_deletion::context::DeletionContext {
            file_set_id,
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
            fs_ops: self.fs_ops.clone(),
            deletion_results: HashMap::new(),
        };

        let pipeline = DeletionPipeline::new();
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

    /// Example test demonstrating how to use MockFileSystemOps
    ///
    /// This test shows the basic pattern for testing file deletion:
    /// 1. Create a mock file system
    /// 2. Add files that should exist
    /// 3. Create the service with the mock
    /// 4. Call the method under test
    /// 5. Verify the mock's state (files deleted, errors handled, etc.)
    ///
    /// Note: This is a template test. To make it work, you'd need to:
    /// - Set up a test database with RepositoryManager
    /// - Create test data (file sets, file infos, etc.)
    /// - Handle the async test setup properly
    #[async_std::test]
    #[ignore] // Ignored because it needs full database setup
    async fn test_delete_file_set_with_mock_fs() {
        // Example of how to use the mock:
        let mock_fs = Arc::new(MockFileSystemOps::new());

        // Add files that should exist in the mock file system
        mock_fs.add_file("/test/rom/game1.zst");
        mock_fs.add_file("/test/rom/game2.zst");

        let test_db_pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(test_db_pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });

        let service =
            FileSetDeletionService::new_with_fs_ops(repository_manager, settings, mock_fs.clone());

        // You would create the service with the mock:
        // let service = FileSetDeletionService::new_with_fs_ops(
        //     repo_manager,
        //     settings,
        //     mock_fs.clone(),
        // );

        // Call the method under test:
        // service.delete_file_set(file_set_id).await.unwrap();

        // Verify files were deleted:
        // assert!(mock_fs.was_deleted("/test/rom/game1.zst"));
        // assert_eq!(mock_fs.get_deleted_files().len(), 1);
    }

    /// Example test showing how to simulate file deletion failures
    #[test]
    #[ignore] // Ignored because it needs full database setup
    fn test_delete_file_set_handles_deletion_failure() {
        // Example of simulating failure:
        let mock_fs = Arc::new(MockFileSystemOps::new());
        mock_fs.add_file("/test/rom/game.zst");

        // Make the deletion fail
        mock_fs.fail_delete_with("Permission denied");

        // The service should log the error and continue
        // (not fail the entire operation)

        // You would verify that:
        // - The error was logged (currently uses eprintln!)
        // - The file_info was NOT deleted from the database
        // - The operation continued for other files
    }

    /// Example test demonstrating the hybrid pipeline pattern (v2)
    #[test]
    #[ignore] // Ignored because it needs full database setup
    fn test_delete_file_set_v2_with_pipeline() {
        // Example of using the new pipeline-based deletion:
        let mock_fs = Arc::new(MockFileSystemOps::new());
        mock_fs.add_file("/test/rom/game1.zst");
        mock_fs.add_file("/test/rom/game2.zst");

        // You would create the service with the mock:
        // let service = FileSetDeletionService::new_with_fs_ops(
        //     repo_manager,
        //     settings,
        //     mock_fs.clone(),
        // );

        // Use the new pipeline version:
        // service.delete_file_set_v2(file_set_id).await.unwrap();

        // Benefits of pipeline version:
        // 1. Each step is isolated and testable
        // 2. Steps can be conditionally executed (should_execute)
        // 3. Clear separation of concerns
        // 4. Easy to add logging/metrics between steps
        // 5. Returns detailed results (deletion_results in context)

        // Verify files were deleted:
        // assert!(mock_fs.was_deleted("/test/rom/game1.zst"));
        // assert!(mock_fs.was_deleted("/test/rom/game2.zst"));
    }
}
