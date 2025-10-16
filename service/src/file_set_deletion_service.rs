use std::sync::Arc;

use core_types::FileSyncStatus;
use database::{models::FileInfo, repository_manager::RepositoryManager};

use crate::{
    error::Error,
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    view_models::Settings,
};

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

// ============================================================================
// Hybrid Pipeline Pattern Implementation
// ============================================================================

/// Context object that flows through the pipeline, accumulating state
pub struct DeletionContext<F: FileSystemOps> {
    pub file_set_id: i64,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<F>,

    // Accumulated state as pipeline progresses
    pub files_to_delete: Vec<FileInfo>,
    pub deletion_results: Vec<FileDeletionResult>,
}

#[derive(Debug, Clone)]
pub struct FileDeletionResult {
    pub file_path: String,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Result of executing a pipeline step
#[derive(Debug)]
pub enum StepAction {
    /// Continue to the next step
    Continue,
    /// Skip all remaining steps (successful early exit)
    Skip,
    /// Abort the pipeline with an error
    Abort(Error),
}

/// Trait for pipeline steps in the deletion process
#[async_trait::async_trait]
pub trait DeletionStep<F: FileSystemOps>: Send + Sync {
    /// Returns the name of this step (for logging/debugging)
    fn name(&self) -> &'static str;

    /// Determines if this step should execute based on current context
    fn should_execute(&self, _context: &DeletionContext<F>) -> bool {
        true // By default, always execute
    }

    /// Execute the step, modifying the context and returning the next action
    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error>;
}

// ============================================================================
// Individual Pipeline Steps
// ============================================================================

/// Step 1: Validate that the file set is not in use by any releases
struct ValidateNotInUseStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for ValidateNotInUseStep {
    fn name(&self) -> &'static str {
        "validate_not_in_use"
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        if context
            .repository_manager
            .get_file_set_repository()
            .is_in_use(context.file_set_id)
            .await?
        {
            return Ok(StepAction::Abort(Error::DbError(
                "File set is in use by one or more releases".to_string(),
            )));
        }
        Ok(StepAction::Continue)
    }
}

/// Step 2: Fetch all file infos for the file set
struct FetchFileInfosStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for FetchFileInfosStep {
    fn name(&self) -> &'static str {
        "fetch_file_infos"
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        let file_infos = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(context.file_set_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        // Store in context for next steps
        context.files_to_delete = file_infos;
        Ok(StepAction::Continue)
    }
}

/// Step 3: Filter files that are only in this file set (safe to delete)
struct FilterDeletableFilesStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for FilterDeletableFilesStep {
    fn name(&self) -> &'static str {
        "filter_deletable_files"
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        let mut deletable_files = vec![];

        for file_info in &context.files_to_delete {
            let file_sets = context
                .repository_manager
                .get_file_set_repository()
                .get_file_sets_by_file_info(file_info.id)
                .await?;

            // Only delete if file is used in exactly this one file set
            if let [entry] = &file_sets[..] {
                if entry.id == context.file_set_id {
                    deletable_files.push(file_info.clone());
                }
            }
        }

        context.files_to_delete = deletable_files;
        Ok(StepAction::Continue)
    }
}

/// Step 4: Mark files for cloud deletion (if synced)
struct MarkForCloudDeletionStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for MarkForCloudDeletionStep {
    fn name(&self) -> &'static str {
        "mark_for_cloud_deletion"
    }

    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        // Only execute if there are files to process
        !context.files_to_delete.is_empty()
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        for file_info in &context.files_to_delete {
            let sync_logs = context
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_by_file_info(file_info.id)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;

            if let Some(entry) = sync_logs.last() {
                context
                    .repository_manager
                    .get_file_sync_log_repository()
                    .add_log_entry(
                        file_info.id,
                        FileSyncStatus::DeletionPending,
                        "",
                        entry.cloud_key.as_str(),
                    )
                    .await
                    .map_err(|e| Error::DbError(e.to_string()))?;
            }
        }

        Ok(StepAction::Continue)
    }
}

/// Step 5: Delete local files and track results
struct DeleteLocalFilesStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for DeleteLocalFilesStep {
    fn name(&self) -> &'static str {
        "delete_local_files"
    }

    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        !context.files_to_delete.is_empty()
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        for file_info in &context.files_to_delete {
            let file_path = context
                .settings
                .get_file_path(&file_info.file_type, &file_info.archive_file_name);

            let path_str = file_path.to_string_lossy().to_string();

            if context.fs_ops.exists(&file_path) {
                match context.fs_ops.remove_file(&file_path) {
                    Ok(_) => {
                        // Successfully deleted - remove from database
                        context
                            .repository_manager
                            .get_file_info_repository()
                            .delete_file_info(file_info.id)
                            .await
                            .map_err(|e| Error::DbError(e.to_string()))?;

                        context.deletion_results.push(FileDeletionResult {
                            file_path: path_str,
                            success: true,
                            error_message: None,
                        });
                    }
                    Err(e) => {
                        // Log error but continue with other files
                        eprintln!(
                            "Failed to delete file: {:?}, error: {}. Continuing with next file.",
                            file_path, e
                        );

                        context.deletion_results.push(FileDeletionResult {
                            file_path: path_str,
                            success: false,
                            error_message: Some(e.to_string()),
                        });
                    }
                }
            }
        }

        Ok(StepAction::Continue)
    }
}

/// Step 6: Delete the file set from database
struct DeleteFileSetStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for DeleteFileSetStep {
    fn name(&self) -> &'static str {
        "delete_file_set"
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<StepAction, Error> {
        context
            .repository_manager
            .get_file_set_repository()
            .delete_file_set(context.file_set_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        Ok(StepAction::Continue)
    }
}

// ============================================================================
// Pipeline Executor
// ============================================================================

struct DeletionPipeline<F: FileSystemOps> {
    steps: Vec<Box<dyn DeletionStep<F>>>,
}

impl<F: FileSystemOps> DeletionPipeline<F> {
    fn new() -> Self {
        Self {
            steps: vec![
                Box::new(ValidateNotInUseStep),
                Box::new(FetchFileInfosStep),
                Box::new(FilterDeletableFilesStep),
                Box::new(MarkForCloudDeletionStep),
                Box::new(DeleteLocalFilesStep),
                Box::new(DeleteFileSetStep),
            ],
        }
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> Result<(), Error> {
        for step in &self.steps {
            // Check if step should execute
            if !step.should_execute(context) {
                eprintln!("Skipping step: {}", step.name());
                continue;
            }

            eprintln!("Executing step: {}", step.name());

            match step.execute(context).await? {
                StepAction::Continue => {
                    // Proceed to next step
                    continue;
                }
                StepAction::Skip => {
                    // Early successful exit
                    eprintln!("Step {} requested skip - stopping pipeline", step.name());
                    return Ok(());
                }
                StepAction::Abort(error) => {
                    // Error exit
                    eprintln!("Step {} requested abort - stopping pipeline", step.name());
                    return Err(error);
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// Service Implementation
// ============================================================================

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

    /// Delete a file set using the hybrid pipeline pattern
    pub async fn delete_file_set_v2(&self, file_set_id: i64) -> Result<(), Error> {
        let mut context = DeletionContext {
            file_set_id,
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
            fs_ops: self.fs_ops.clone(),
            files_to_delete: vec![],
            deletion_results: vec![],
        };

        let pipeline = DeletionPipeline::new();
        pipeline.execute(&mut context).await?;

        // You can now access deletion results if needed
        let successful_deletions = context
            .deletion_results
            .iter()
            .filter(|r| r.success)
            .count();
        let failed_deletions = context
            .deletion_results
            .iter()
            .filter(|r| !r.success)
            .count();

        eprintln!(
            "Deletion complete: {} successful, {} failed",
            successful_deletions, failed_deletions
        );

        Ok(())
    }

    pub async fn delete_file_set(&self, file_set_id: i64) -> Result<(), Error> {
        // First check if file set is in use by any releases

        if self
            .repository_manager
            .get_file_set_repository()
            .is_in_use(file_set_id)
            .await?
        {
            return Err(Error::DbError(
                "File set is in use by one or more releases".to_string(),
            ));
        }

        // If not in use, then fetch the file set file infos from database

        let file_set_file_info = self
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        // For each file in file set, check if it is used in other file sets
        // If not, collect the file for deletion

        let mut file_infos_for_deletion = vec![];

        for file_info in file_set_file_info {
            let res = self
                .repository_manager
                .get_file_set_repository()
                .get_file_sets_by_file_info(file_info.id)
                .await?;
            if let [entry] = &res[..] {
                // exactly one entry
                if entry.id == file_set_id {
                    file_infos_for_deletion.push(file_info);
                }
            }
        }

        // Go through the file infos to delete
        for file_info in file_infos_for_deletion {
            // - check for file sync entries from db, if file is synced mark it for deletion
            let res = self
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_by_file_info(file_info.id)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
            if let Some(entry) = res.last() {
                self.repository_manager
                    .get_file_sync_log_repository()
                    .add_log_entry(
                        file_info.id,
                        FileSyncStatus::DeletionPending,
                        "",
                        entry.cloud_key.as_str(),
                    )
                    .await
                    .map_err(|e| Error::DbError(e.to_string()))?;
            }

            // - check if file exists in local storage and delete it
            let file_path = self
                .settings
                .get_file_path(&file_info.file_type, &file_info.archive_file_name);

            if self.fs_ops.exists(&file_path) {
                if let Err(e) = self.fs_ops.remove_file(&file_path) {
                    //   - if there's a failure in deletion, log it and continue
                    eprintln!(
                        "Failed to delete file: {:?}, error: {}. Continuing with next file.",
                        file_path, e
                    );
                } else {
                    //   - if the deletion was successful, remove the file info from db
                    //   TODO:
                    //   -- ensure that file_set_file_info link entry will be deleted also
                    //   -- ensure that file_info_system link entry will be deleted also
                    self.repository_manager
                        .get_file_info_repository()
                        .delete_file_info(file_info.id)
                        .await
                        .map_err(|e| Error::DbError(e.to_string()))?;
                }
            }
        }

        // unlink the file set from any releases

        // When all file infos are processed, delete the file set from db
        // TODO:
        // -- ensure that release_file_set link entry will be deleted also
        self.repository_manager
            .get_file_set_repository()
            .delete_file_set(file_set_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;
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

    #[async_std::test]
    async fn test_validate_not_in_use_step() {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings::default());
        let fs_ops = Arc::new(MockFileSystemOps::new());

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

        let file2 = ImportedFile {
            original_file_name: "file2.zst".to_string(),
            archive_file_name: "file2.zst".to_string(),
            sha1_checksum: Sha1Checksum::from([1; 20]),
            file_size: 5678,
        };

        let files_in_file_set = vec![file1, file2];
        let file_set_id = repo_manager
            .get_file_set_repository()
            .add_file_set(
                "test_set",
                "file name",
                &FileType::Rom,
                "",
                &files_in_file_set,
                &[system_id],
            )
            .await
            .unwrap();

        let software_title_id = repo_manager
            .get_software_title_repository()
            .add_software_title("Test Software", None)
            .await
            .unwrap();

        // link file set to release
        let release_id = repo_manager
            .get_release_repository()
            .add_release_full(
                "Test Release",
                &[software_title_id],
                &[file_set_id],
                &[system_id],
            )
            .await
            .unwrap();

        let mut context = DeletionContext {
            file_set_id: 1,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            files_to_delete: vec![],
            deletion_results: vec![],
        };

        let step = ValidateNotInUseStep;

        let action = step.execute(&mut context).await.unwrap();
        assert!(matches!(action, StepAction::Abort(_)));

        // Delete release - link to file set should have been deleted also and file set can be
        // deleted now
        repo_manager
            .get_release_repository()
            .delete_release(release_id)
            .await
            .unwrap();

        let action = step.execute(&mut context).await.unwrap();
        assert!(matches!(action, StepAction::Continue));
    }
}
