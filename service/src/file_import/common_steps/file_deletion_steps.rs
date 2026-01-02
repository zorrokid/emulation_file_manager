use std::sync::Arc;

use core_types::FileSyncStatus;
use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    file_set_deletion::model::FileDeletionResult,
    file_system_ops::FileSystemOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
    view_models::Settings,
};

pub trait FileDeletionStepsContext {
    fn repository_manager(&self) -> Arc<RepositoryManager>;
    fn file_set_id(&self) -> i64;
    fn has_deletion_candidates(&self) -> bool {
        !self.deletion_results().is_empty()
    }
    fn has_deletable_files(&self) -> bool {
        self.deletion_results().values().any(|r| r.is_deletable)
    }
    fn has_deleted_files(&self) -> bool {
        self.deletion_results()
            .values()
            .any(|r| r.is_deletable && r.file_deletion_success.is_some_and(|s| s))
    }
    fn deletion_results(&self) -> &std::collections::HashMap<Vec<u8>, FileDeletionResult>;
    fn deletion_results_mut(
        &mut self,
    ) -> &mut std::collections::HashMap<Vec<u8>, FileDeletionResult>;
    fn settings(&self) -> Arc<Settings>;
    fn fs_ops(&self) -> Arc<dyn FileSystemOps>;
}

/// Filter files that are only in this file set (safe to delete)
pub struct FilterDeletableFilesStep<T: FileDeletionStepsContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: FileDeletionStepsContext> Default for FilterDeletableFilesStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: FileDeletionStepsContext> FilterDeletableFilesStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: FileDeletionStepsContext + Send + Sync> PipelineStep<T> for FilterDeletableFilesStep<T> {
    fn name(&self) -> &'static str {
        "filter_deletable_files"
    }

    fn should_execute(&self, context: &T) -> bool {
        // Only execute if there are files to process
        context.has_deletion_candidates()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        println!(
            "Filtering deletable files for file set {}",
            context.file_set_id()
        );
        tracing::info!(
            "Filtering deletable files for file set {}",
            context.file_set_id()
        );
        let file_set_id = context.file_set_id();
        let repository_manager = context.repository_manager();
        for deletion_result in context.deletion_results_mut().values_mut() {
            tracing::info!(
                "Checking if file info with id {} is deletable",
                deletion_result.file_info.id
            );

            let file_sets_res = repository_manager
                .get_file_set_repository()
                .get_file_sets_by_file_info(deletion_result.file_info.id)
                .await;

            match file_sets_res {
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch file sets for file info with id {}: {}",
                        deletion_result.file_info.id,
                        e
                    );
                    return StepAction::Abort(Error::DbError(format!(
                        "Failed to fetch file sets for file info with id {}: {}",
                        deletion_result.file_info.id, e
                    )));
                }
                Ok(file_sets) => {
                    tracing::info!(
                        "File info with id {} is used in {} file sets",
                        deletion_result.file_info.id,
                        file_sets.len()
                    );
                    // Only delete if file is used in exactly this one file set
                    if let [single_entry] = &file_sets[..]
                        && single_entry.id == file_set_id
                    {
                        tracing::info!(
                            "File info with id {} is only used in file set with id {}, marking as deletable",
                            deletion_result.file_info.id,
                            file_set_id
                        );
                        deletion_result.is_deletable = true;
                    }
                }
            }
        }

        StepAction::Continue
    }
}

pub struct DeleteLocalFilesStep<T: FileDeletionStepsContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: FileDeletionStepsContext> Default for DeleteLocalFilesStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: FileDeletionStepsContext> DeleteLocalFilesStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: FileDeletionStepsContext + Send + Sync> PipelineStep<T> for DeleteLocalFilesStep<T> {
    fn name(&self) -> &'static str {
        "delete_local_files"
    }

    fn should_execute(&self, context: &T) -> bool {
        context.has_deletable_files()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        println!(
            "Deleting local files for file set with id {}",
            context.file_set_id()
        );
        tracing::info!(
            "Deleting local files for file set with id {}",
            context.file_set_id()
        );

        let settings = context.settings();
        let fs_ops = context.fs_ops();

        for deletion_result in context
            .deletion_results_mut()
            .values_mut()
            .filter(|f| f.is_deletable)
        {
            println!(
                "Processing file info with id {} and name {} for local deletion",
                deletion_result.file_info.id, deletion_result.file_info.archive_file_name
            );
            tracing::info!(
                "Processing file info with id {} for local deletion",
                deletion_result.file_info.id
            );
            let file_path = settings.get_file_path(
                &deletion_result.file_info.file_type,
                &deletion_result.file_info.archive_file_name,
            );

            let path_str = file_path.to_string_lossy().to_string();
            println!("Resolved file path: {}", path_str);
            tracing::info!(
                "Resolved file path for file info id {}: {}",
                deletion_result.file_info.id,
                path_str
            );
            deletion_result.file_path = Some(path_str.clone());

            tracing::info!("Attempting to delete local file: {}", path_str);

            if fs_ops.exists(&file_path) {
                println!("File exists, proceeding with deletion: {}", path_str);
                tracing::info!("File exists, proceeding with deletion: {}", path_str);
                match fs_ops.remove_file(&file_path) {
                    Ok(_) => {
                        tracing::info!("Deleted local file: {}", path_str);
                        deletion_result.file_deletion_success = Some(true);
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete local file {}: {}", path_str, e);
                        deletion_result.file_deletion_success = Some(false);
                        deletion_result.error_messages.push(e.to_string());
                    }
                }
            } else {
                println!("File {} does not exist, skipping deletion.", path_str);
                tracing::info!("File {} does not exist, skipping deletion.", path_str);
                deletion_result.file_deletion_success = Some(true); // consider non-existing file as "deleted" (user might have done it manually)
            }
        }

        StepAction::Continue
    }
}

/// Mark files for cloud deletion (if synced to cloud)
/// We don't delete from cloud here, just mark them for deletion in the sync logs.
/// The reason is the cloud deletion needs an internet connection but file set deletion should work
/// offline.
pub struct MarkForCloudDeletionStep<T: FileDeletionStepsContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: FileDeletionStepsContext> Default for MarkForCloudDeletionStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: FileDeletionStepsContext> MarkForCloudDeletionStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: FileDeletionStepsContext + Send + Sync> PipelineStep<T> for MarkForCloudDeletionStep<T> {
    fn name(&self) -> &'static str {
        "mark_for_cloud_deletion"
    }

    fn should_execute(&self, context: &T) -> bool {
        // No need to check if cloud sync is enabled here
        // - if sync has been enabled at some point and files were synced, we need to mark them for
        // deletion anyway, next time the sync is enabled and triggered, the files marked
        // for deletion will be processed.
        //
        // // Only execute if there are deletable files to process
        context.has_deleted_files()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        println!(
            "Marking files for cloud deletion for file set with id {}",
            context.file_set_id()
        );
        tracing::info!(
            "Marking files for cloud deletion for file set with id {}",
            context.file_set_id()
        );
        let repository_manager = context.repository_manager();
        for deletion_result in context
            .deletion_results_mut()
            .values_mut()
            .filter(|f| f.is_deletable && f.file_deletion_success.is_some_and(|s| s))
        {
            let sync_logs_res = repository_manager
                .get_file_sync_log_repository()
                .get_logs_by_file_info(deletion_result.file_info.id)
                .await;

            match sync_logs_res {
                Err(e) => {
                    tracing::error!(
                        "Failed to fetch sync logs for file info with id {}: {}",
                        deletion_result.file_info.id,
                        e
                    );
                    return StepAction::Abort(Error::DbError(format!(
                        "Failed to fetch sync logs for file info with id {}: {}",
                        deletion_result.file_info.id, e
                    )));
                }
                Ok(sync_logs) => {
                    tracing::info!(
                        "Fetched sync logs for file info with id {}",
                        deletion_result.file_info.id
                    );

                    if let Some(entry) = sync_logs.last() {
                        tracing::info!(
                            "File info with id {} has last sync log with status {:?}, marking for cloud deletion",
                            deletion_result.file_info.id,
                            entry.status
                        );

                        let update_res = repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                deletion_result.file_info.id,
                                FileSyncStatus::DeletionPending,
                                "",
                                entry.cloud_key.as_str(),
                            )
                            .await;
                        if let Err(e) = update_res {
                            // TODO: should this abort?
                            deletion_result.cloud_delete_marked_successfully = Some(false);
                            tracing::error!(
                                "Failed to mark file info with id {} for cloud deletion: {}",
                                deletion_result.file_info.id,
                                e
                            );
                            return StepAction::Abort(Error::DbError(format!(
                                "Failed to mark file info with id {} for cloud deletion: {}",
                                deletion_result.file_info.id, e
                            )));
                        }
                        deletion_result.cloud_delete_marked_successfully = Some(true);
                    }
                }
            }
        }

        StepAction::Continue
    }
}

/// Delete file_info entries for deleted files from database
pub struct DeleteFileInfosStep<T: FileDeletionStepsContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: FileDeletionStepsContext> Default for DeleteFileInfosStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: FileDeletionStepsContext> DeleteFileInfosStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: FileDeletionStepsContext + Send + Sync> PipelineStep<T> for DeleteFileInfosStep<T> {
    fn name(&self) -> &'static str {
        "delete_file_info_entries"
    }

    fn should_execute(&self, context: &T) -> bool {
        context.has_deleted_files()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        println!(
            "Deleting file_info entries for file set {}",
            context.file_set_id()
        );
        tracing::info!(
            "Deleting file_info entries for file set {}",
            context.file_set_id()
        );
        let repository_manager = context.repository_manager();
        for dr in context
            .deletion_results_mut()
            .values_mut()
            .filter(|f| f.is_deletable && f.file_deletion_success.is_some_and(|s| s))
        {
            tracing::info!(
                "Processing file_info with id {} for deletion",
                dr.file_info.id
            );
            let delete_res = repository_manager
                .get_file_info_repository()
                .delete_file_info(dr.file_info.id)
                .await;
            match delete_res {
                Ok(_) => {
                    tracing::info!("Deleted file_info with id {} from DB", dr.file_info.id);
                    dr.db_deletion_success = Some(true);
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to delete file_info with id {} from DB: {}",
                        dr.file_info.id,
                        e
                    );
                    dr.db_deletion_success = Some(false);
                    dr.error_messages.push(format!(
                        "Failed to delete file_info with id {} from DB: {}",
                        dr.file_info.id, e
                    ));
                }
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::{FileSyncStatus, FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        file_import::common_steps::file_deletion_steps::{
            DeleteFileInfosStep, DeleteLocalFilesStep, FileDeletionStepsContext,
            FilterDeletableFilesStep, MarkForCloudDeletionStep,
        },
        file_set_deletion::model::FileDeletionResult,
        file_system_ops::{FileSystemOps, mock::MockFileSystemOps},
        pipeline::pipeline_step::{PipelineStep, StepAction},
        view_models::Settings,
    };

    struct TestContext {
        file_set_id: i64,
        repository_manager: Arc<RepositoryManager>,
        deletion_results: HashMap<Vec<u8>, FileDeletionResult>,
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
    }

    impl FileDeletionStepsContext for TestContext {
        fn repository_manager(&self) -> Arc<RepositoryManager> {
            self.repository_manager.clone()
        }

        fn file_set_id(&self) -> i64 {
            self.file_set_id
        }

        fn deletion_results_mut(&mut self) -> &mut HashMap<Vec<u8>, FileDeletionResult> {
            &mut self.deletion_results
        }

        fn deletion_results(&self) -> &HashMap<Vec<u8>, FileDeletionResult> {
            &self.deletion_results
        }

        fn settings(&self) -> Arc<Settings> {
            self.settings.clone()
        }

        fn fs_ops(&self) -> Arc<dyn crate::file_system_ops::FileSystemOps> {
            self.fs_ops.clone()
        }
    }

    #[async_std::test]
    async fn test_filter_deletable_files_step() {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
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

        let file2_clone = file2.clone();

        let file_set_id =
            prepare_file_set_with_files(&repo_manager, system_id, &[file1, file2]).await;

        // add another file set that uses file2
        let _another_file_set_id =
            prepare_file_set_with_files(&repo_manager, system_id, &[file2_clone]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        assert_eq!(file_infos.len(), 2);
        let file_info_1 = file_infos
            .iter()
            .find(|fi| fi.archive_file_name == "file1.zst")
            .unwrap();
        let file_info_2 = file_infos
            .iter()
            .find(|fi| fi.archive_file_name == "file2.zst")
            .unwrap();

        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            deletion_results: HashMap::from([
                (
                    file_info_1.sha1_checksum.clone(),
                    FileDeletionResult::new(file_info_1.clone()),
                ),
                (
                    file_info_2.sha1_checksum.clone(),
                    FileDeletionResult::new(file_info_2.clone()),
                ),
            ]),
            settings: Arc::new(Settings::default()),
            fs_ops: Arc::new(MockFileSystemOps::new()),
        };

        let filter_step = FilterDeletableFilesStep::<TestContext>::new();
        filter_step.execute(&mut context).await;

        // only file1 should be deletable
        assert_eq!(
            context
                .deletion_results
                .values()
                .filter(|f| f.is_deletable)
                .count(),
            1
        );
        let deletable_file = context
            .deletion_results
            .values()
            .find(|f| f.is_deletable)
            .unwrap();
        assert_eq!(deletable_file.file_info.archive_file_name, "file1.zst");
    }

    #[async_std::test]
    async fn test_mark_for_cloud_deletion_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        let file_info = file_infos.first().unwrap();

        let file_info_id = file_info.id;

        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadPending,
                "",
                "cloud/key/file.zst",
            )
            .await
            .unwrap();
        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadInProgress,
                "",
                "cloud/key/file.zst",
            )
            .await
            .unwrap();
        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadCompleted,
                "",
                "cloud/key/file.zst",
            )
            .await
            .unwrap();

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        file_deletion_result.file_deletion_success = Some(true);

        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = MarkForCloudDeletionStep::<TestContext>::new();
        step.execute(&mut context).await;
        let logs = repo_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(logs.len(), 4);
        assert_eq!(logs[0].status, FileSyncStatus::DeletionPending);
    }

    #[async_std::test]
    async fn test_mark_for_cloud_deletion_step_with_failed_local_deletion() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        let file_info = file_infos.first().unwrap();

        let file_info_id = file_info.id;

        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadPending,
                "",
                "cloud/key/file.zst",
            )
            .await
            .unwrap();
        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadInProgress,
                "",
                "cloud/key/file.zst",
            )
            .await
            .unwrap();
        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadCompleted,
                "",
                "cloud/key/file.zst",
            )
            .await
            .unwrap();

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        file_deletion_result.file_deletion_success = Some(false);

        let context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = MarkForCloudDeletionStep::<TestContext>::new();
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_delete_local_files_step_when_file_does_not_exist() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();
        let file_info = file_infos.first().unwrap();

        // Let's not add the file to fs_ops, simulating that it doesn't exist
        // let file_path = settings.get_file_path(&file_info.file_type, &file_info.archive_file_name);
        // fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = DeleteLocalFilesStep::<TestContext>::new();
        let res = step.execute(&mut context).await;

        println!(
            "Deletion result: {:?}",
            context.deletion_results.get(&file_info.sha1_checksum)
        );

        assert!(
            context
                .deletion_results
                .get(&file_info.sha1_checksum)
                .unwrap()
                .file_deletion_success
                .unwrap()
        );

        assert_eq!(res, StepAction::Continue);
    }

    #[async_std::test]
    async fn test_delete_local_files_step_with_delete_failure() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        fs_ops.fail_delete_with("Permission denied");

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();
        let file_info = file_infos.first().unwrap();
        let file_path = settings.get_file_path(&file_info.file_type, &file_info.archive_file_name);
        println!(
            "Adding file to mock FS ops: {}",
            file_path.to_string_lossy()
        );
        fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = DeleteLocalFilesStep::<TestContext>::new();
        let res = step.execute(&mut context).await;

        assert!(
            !context
                .deletion_results
                .get(&file_info.sha1_checksum)
                .unwrap()
                .file_deletion_success
                .unwrap()
        );

        assert_eq!(res, StepAction::Continue);
    }

    #[async_std::test]
    async fn test_delete_local_files_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;
        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();
        let file_info = file_infos.first().unwrap();
        let file_path = settings.get_file_path(&file_info.file_type, &file_info.archive_file_name);

        println!(
            "Adding file to mock FS ops: {}",
            file_path.to_string_lossy()
        );
        fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;

        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = DeleteLocalFilesStep::<TestContext>::new();
        let res = step.execute(&mut context).await;
        let fp = settings.get_file_path(&file_info.file_type, &file_info.archive_file_name);
        println!("Checking if file was deleted: {}", fp.to_string_lossy());
        assert!(fs_ops.was_deleted(fp.to_string_lossy().as_ref()));

        println!(
            "Deletion result: {:?}",
            context.deletion_results.get(&file_info.sha1_checksum)
        );

        assert!(
            context
                .deletion_results
                .get(&file_info.sha1_checksum)
                .unwrap()
                .file_deletion_success
                .unwrap()
        );

        assert_eq!(res, StepAction::Continue);
    }

    #[async_std::test]
    async fn test_delete_file_infos_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        let file_info = file_infos.first().unwrap();

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        file_deletion_result.file_deletion_success = Some(true);
        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };

        // delete file set first to remove the link
        repo_manager
            .get_file_set_repository()
            .delete_file_set(file_set_id)
            .await
            .unwrap();

        // TODO:: does this work without these?
        //let delete_file_set_step = DeleteFileSetStep;
        //delete_file_set_step.execute(&mut context).await;

        let step = DeleteFileInfosStep::<TestContext>::new();
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);

        let deletion_result = context
            .deletion_results
            .get(&file_info.sha1_checksum)
            .unwrap();

        assert!(deletion_result.db_deletion_success.unwrap());

        let res = repo_manager
            .get_file_info_repository()
            .get_file_info(file_info.id)
            .await;
        assert!(res.is_err());
    }

    #[async_std::test]
    async fn test_delete_file_infos_step_file_is_linked_to_file_set() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        let file_info = file_infos.first().unwrap();

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        file_deletion_result.file_deletion_success = Some(true);
        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };

        let step = DeleteFileInfosStep::<TestContext>::new();
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);

        let deletion_result = context
            .deletion_results
            .get(&file_info.sha1_checksum)
            .unwrap();

        assert!(!deletion_result.db_deletion_success.unwrap());

        let res = repo_manager
            .get_file_info_repository()
            .get_file_info(file_info.id)
            .await;
        assert!(res.is_ok());
    }

    #[async_std::test]
    async fn test_delete_file_infos_step_with_file_deletion_failed() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        let file_info = file_infos.first().unwrap();

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        file_deletion_result.file_deletion_success = Some(false);
        let context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };

        let delete_file_infos_step = DeleteFileInfosStep::<TestContext>::new();
        assert!(!delete_file_infos_step.should_execute(&context));
    }

    struct TestSetup {
        settings: Arc<Settings>,
        repo_manager: Arc<RepositoryManager>,
        fs_ops: Arc<MockFileSystemOps>,
        system_id: i64,
        file1: ImportedFile,
    }

    async fn prepare_test() -> TestSetup {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });
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

        TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        }
    }

    async fn prepare_file_set_with_files(
        repo_manager: &RepositoryManager,
        system_id: i64,
        files: &[ImportedFile],
    ) -> i64 {
        repo_manager
            .get_file_set_repository()
            .add_file_set(
                "test_set",
                "file name",
                &FileType::Rom,
                "",
                files,
                &[system_id],
            )
            .await
            .unwrap()
    }
}
