use std::sync::Arc;

use core_types::{CloudSyncStatus, Sha1Checksum};
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
    fn deletion_results(&self) -> &std::collections::HashMap<Sha1Checksum, FileDeletionResult>;
    fn deletion_results_mut(
        &mut self,
    ) -> &mut std::collections::HashMap<Sha1Checksum, FileDeletionResult>;
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
        tracing::info!(
            file_set_id = context.file_set_id(),
            "Filtering deletable files for file set",
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
        tracing::info!(
            file_set_id = context.file_set_id(),
            "Deleting local files for file set"
        );

        let settings = context.settings();
        let fs_ops = context.fs_ops();

        for deletion_result in context
            .deletion_results_mut()
            .values_mut()
            .filter(|f| f.is_deletable)
        {
            tracing::info!(
                file_info_id = deletion_result.file_info.id,
                "Processing file info for local deletion"
            );
            let Some(archive_name) = &deletion_result.file_info.archive_file_name else {
                tracing::warn!(
                    file_info_id = deletion_result.file_info.id,
                    "File info does not have an archive file name, skipping local deletion.",
                );
                continue;
            };
            let file_path =
                settings.get_file_path(&deletion_result.file_info.file_type, archive_name);

            let path_str = file_path.to_string_lossy().to_string();
            tracing::info!(
                file_info_id = deletion_result.file_info.id,
                path = path_str.as_str(),
                "Resolved file path for file info",
            );
            deletion_result.file_path = Some(path_str.clone());

            tracing::info!(path = path_str.as_str(), "Attempting to delete local file");

            if fs_ops.exists(&file_path) {
                tracing::info!(
                    path = path_str.as_str(),
                    "File exists, proceeding with deletion"
                );
                match fs_ops.remove_file(&file_path) {
                    Ok(_) => {
                        tracing::info!(path = path_str.as_str(), "Deleted local file");
                        deletion_result.file_deletion_success = Some(true);
                    }
                    Err(e) => {
                        tracing::error!( path = path_str.as_str(), error = %e, "Failed to delete local file");
                        deletion_result.file_deletion_success = Some(false);
                        deletion_result.error_messages.push(e.to_string());
                    }
                }
            } else {
                tracing::info!(
                    path = path_str.as_str(),
                    "File does not exist, skipping deletion."
                );
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
        tracing::info!(
            file_set_id = context.file_set_id(),
            "Marking files for cloud deletion",
        );
        let repository_manager = context.repository_manager();
        for deletion_result in context
            .deletion_results_mut()
            .values_mut()
            .filter(|f| f.is_deletable && f.file_deletion_success.is_some_and(|s| s))
        {
            match deletion_result.file_info.cloud_sync_status {
                CloudSyncStatus::Synced => {
                    let update_res = repository_manager
                        .get_file_info_repository()
                        .update_cloud_sync_status(
                            deletion_result.file_info.id,
                            CloudSyncStatus::DeletionPending,
                        )
                        .await;
                    match update_res {
                        Ok(_) => {
                            // Update in-memory status so DeleteFileInfosStep skips this tombstone
                            deletion_result.file_info.cloud_sync_status =
                                CloudSyncStatus::DeletionPending;
                            deletion_result.cloud_delete_marked_successfully = Some(true);
                            tracing::info!(
                                file_info_id = deletion_result.file_info.id,
                                "Marked file_info for cloud deletion",
                            );
                        }
                        Err(e) => {
                            deletion_result.cloud_delete_marked_successfully = Some(false);
                            tracing::error!(
                                file_info_id = deletion_result.file_info.id,
                                error = %e,
                                "Failed to mark file_info for cloud deletion",
                            );
                            return StepAction::Abort(Error::DbError(format!(
                                "Failed to mark file_info with id {} for cloud deletion: {}",
                                deletion_result.file_info.id, e
                            )));
                        }
                    }
                }
                CloudSyncStatus::NotSynced | CloudSyncStatus::DeletionPending => {
                    // Nothing to do: file was never synced or already marked for deletion
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
        tracing::info!(
            "Deleting file_info entries for file set {}",
            context.file_set_id()
        );
        let repository_manager = context.repository_manager();
        for dr in context
            .deletion_results_mut()
            .values_mut()
            .filter(|f| {
                f.is_deletable
                    && f.file_deletion_success.is_some_and(|s| s)
                    && f.file_info.cloud_sync_status == CloudSyncStatus::NotSynced
            })
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

    use core_types::{CloudSyncStatus, FileType, ImportedFile, Sha1Checksum};
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
        deletion_results: HashMap<Sha1Checksum, FileDeletionResult>,
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

        fn deletion_results_mut(&mut self) -> &mut HashMap<Sha1Checksum, FileDeletionResult> {
            &mut self.deletion_results
        }

        fn deletion_results(&self) -> &HashMap<Sha1Checksum, FileDeletionResult> {
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
            archive_file_name: Some("file1.zst".to_string()),
            sha1_checksum: Sha1Checksum::from([0; 20]),
            file_size: 1234,
            is_available: true,
        };

        let file2 = ImportedFile {
            original_file_name: "file2.zst".to_string(),
            archive_file_name: Some("file2.zst".to_string()),
            sha1_checksum: Sha1Checksum::from([1; 20]),
            file_size: 5678,
            is_available: true,
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
            .find(|fi| fi.archive_file_name == Some("file1.zst".to_string()))
            .unwrap();
        let file_info_2 = file_infos
            .iter()
            .find(|fi| fi.archive_file_name == Some("file2.zst".to_string()))
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
        assert_eq!(
            deletable_file.file_info.archive_file_name,
            Some("file1.zst".to_string())
        );
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

        // Mark as synced so the step has something to act on
        repo_manager
            .get_file_info_repository()
            .update_cloud_sync_status(file_info.id, CloudSyncStatus::Synced)
            .await
            .unwrap();

        let mut file_info_synced = file_info.clone();
        file_info_synced.cloud_sync_status = CloudSyncStatus::Synced;

        let mut file_deletion_result = FileDeletionResult::new(file_info_synced);
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

        let updated = repo_manager
            .get_file_info_repository()
            .get_file_info(file_info.id)
            .await
            .unwrap();
        assert_eq!(updated.cloud_sync_status, CloudSyncStatus::DeletionPending);
    }

    #[async_std::test]
    async fn test_mark_for_cloud_deletion_step_not_synced_file_is_not_marked() {
        // Files that were never synced should not be marked for deletion
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
        // cloud_sync_status defaults to NotSynced

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

        let updated = repo_manager
            .get_file_info_repository()
            .get_file_info(file_info.id)
            .await
            .unwrap();
        // Status should remain NotSynced
        assert_eq!(updated.cloud_sync_status, CloudSyncStatus::NotSynced);
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
        let file_path = settings.get_file_path(
            &file_info.file_type,
            file_info.archive_file_name.as_deref().unwrap(),
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
        let file_path = settings.get_file_path(
            &file_info.file_type,
            file_info.archive_file_name.as_deref().unwrap(),
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
        let fp = settings.get_file_path(
            &file_info.file_type,
            file_info.archive_file_name.as_deref().unwrap(),
        );
        assert!(fs_ops.was_deleted(fp.to_string_lossy().as_ref()));


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
    async fn test_delete_local_files_step_skips_when_archive_file_name_is_none() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            ..
        } = prepare_test().await;

        let unavailable_file = ImportedFile {
            original_file_name: "missing.rom".to_string(),
            archive_file_name: None,
            sha1_checksum: Sha1Checksum::from([1; 20]),
            file_size: 0,
            is_available: false,
        };

        let file_set_id =
            prepare_file_set_with_files(&repo_manager, system_id, &[unavailable_file]).await;
        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();
        let file_info = file_infos.first().unwrap();
        assert!(file_info.archive_file_name.is_none());

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        let checksum = file_info.sha1_checksum.clone();

        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(checksum.clone(), file_deletion_result)]),
        };

        let step = DeleteLocalFilesStep::<TestContext>::new();
        let res = step.execute(&mut context).await;

        // No FS delete should have been attempted
        assert_eq!(fs_ops.get_deleted_files(), vec![] as Vec<String>);
        // file_deletion_success stays None — we neither succeeded nor failed
        assert!(
            context
                .deletion_results
                .get(&checksum)
                .unwrap()
                .file_deletion_success
                .is_none()
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
    async fn test_delete_file_infos_step_skips_deletion_pending() {
        // DeletionPending file_infos are tombstones kept until the cloud deletion runs.
        // DeleteFileInfosStep must not remove them — they would be missed by DeleteMarkedFilesStep.
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

        // Simulate that MarkForCloudDeletionStep already set this to DeletionPending
        repo_manager
            .get_file_info_repository()
            .update_cloud_sync_status(file_info.id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        let mut file_info_pending = file_info.clone();
        file_info_pending.cloud_sync_status = CloudSyncStatus::DeletionPending;

        let mut file_deletion_result = FileDeletionResult::new(file_info_pending);
        file_deletion_result.is_deletable = true;
        file_deletion_result.file_deletion_success = Some(true);

        // Remove file_set link so FK wouldn't prevent a delete if the step tried
        repo_manager
            .get_file_set_repository()
            .delete_file_set(file_set_id)
            .await
            .unwrap();

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

        // file_info must still exist — it's a tombstone for cloud deletion
        let res = repo_manager
            .get_file_info_repository()
            .get_file_info(file_info.id)
            .await;
        assert!(res.is_ok());

        // db_deletion_success remains None because the step skipped this record
        let dr = context
            .deletion_results
            .get(&file_info.sha1_checksum)
            .unwrap();
        assert!(dr.db_deletion_success.is_none());
    }

    #[async_std::test]
    async fn test_delete_file_infos_step_file_is_linked_to_another_file_set() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file1_clone = file1.clone();
        let file_set_id =
            prepare_file_set_with_files(&repo_manager, system_id, &[file1_clone]).await;
        // second file set referencing the file, for this test we wouldn't need to insert this,
        // settings is_deletable to false and file_deletion_success to None should be enough.
        let _ = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        let file_info = file_infos.first().unwrap();

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = false;
        file_deletion_result.file_deletion_success = None;
        let mut context = TestContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(file_info.sha1_checksum, file_deletion_result)]),
        };

        let step = DeleteFileInfosStep::<TestContext>::new();
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);

        let deletion_result = context
            .deletion_results
            .get(&file_info.sha1_checksum)
            .unwrap();

        assert!(deletion_result.db_deletion_success.is_none());

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
            archive_file_name: Some("file1.zst".to_string()),
            sha1_checksum: Sha1Checksum::from([0; 20]),
            file_size: 1234,
            is_available: true,
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
