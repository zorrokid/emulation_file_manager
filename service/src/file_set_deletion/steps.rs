use core_types::FileSyncStatus;

use crate::{
    error::Error,
    file_set_deletion::{context::DeletionContext, model::FileDeletionResult},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

/// Step 1: Validate that the file set is not in use by any releases
pub struct ValidateNotInUseStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for ValidateNotInUseStep {
    fn name(&self) -> &'static str {
        "validate_not_in_use"
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Validating that file set with id {} is not in use",
            context.file_set_id
        );

        let file_set_id = context.file_set_id;

        let is_in_use_res = context
            .repository_manager
            .get_file_set_repository()
            .is_in_use(file_set_id)
            .await;
        match is_in_use_res {
            Err(e) => StepAction::Abort(Error::DbError(format!(
                "Failed to check if file set with id {} is in use: {}",
                file_set_id, e
            ))),
            Ok(in_use) => {
                if in_use {
                    tracing::warn!(
                        "File set with id {} is in use by one or more releases, aborting deletion",
                        file_set_id
                    );
                    StepAction::Abort(Error::DbError(
                        "File set is in use by one or more releases".to_string(),
                    ))
                } else {
                    tracing::info!(
                        "File set with id {} is not in use, proceeding with deletion",
                        file_set_id
                    );
                    StepAction::Continue
                }
            }
        }
    }
}

/// Step 2: Fetch all file infos for the file set
pub struct FetchFileInfosStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for FetchFileInfosStep {
    fn name(&self) -> &'static str {
        "fetch_file_infos"
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Fetching file infos for file set with id {}",
            context.file_set_id
        );

        let file_infos_res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(context.file_set_id)
            .await;

        match file_infos_res {
            Ok(file_infos) => {
                tracing::info!(
                    "Fetched {} file infos for file set {}",
                    file_infos.len(),
                    context.file_set_id
                );
                // even if file_infos is empty, continue to delete the file set
                context.deletion_results = file_infos
                    .into_iter()
                    .map(|fi| {
                        tracing::info!("Found file info with id {} for deletion", fi.id);
                        (fi.sha1_checksum.clone(), FileDeletionResult::new(fi))
                    })
                    .collect();

                StepAction::Continue
            }
            Err(e) => {
                tracing::error!(
                    "Failed to fetch file infos for file set {}: {}",
                    context.file_set_id,
                    e
                );
                StepAction::Abort(Error::DbError(format!("Failed to fetch file infos: {}", e)))
            }
        }
    }
}

/// Step 3: Filter files that are only in this file set (safe to delete)
pub struct FilterDeletableFilesStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for FilterDeletableFilesStep {
    fn name(&self) -> &'static str {
        "filter_deletable_files"
    }

    fn should_execute(&self, context: &DeletionContext) -> bool {
        // Only execute if there are files to process
        !context.deletion_results.is_empty()
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Filtering deletable files for file set {}",
            context.file_set_id
        );
        for deletion_result in context.deletion_results.values_mut() {
            tracing::info!(
                "Checking if file info with id {} is deletable",
                deletion_result.file_info.id
            );

            let file_sets_res = context
                .repository_manager
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
                        && single_entry.id == context.file_set_id
                    {
                        tracing::info!(
                            "File info with id {} is only used in file set with id {}, marking as deletable",
                            deletion_result.file_info.id,
                            context.file_set_id
                        );
                        deletion_result.is_deletable = true;
                    }
                }
            }
        }

        StepAction::Continue
    }
}

/// Step 4: Delete the file set from database
pub struct DeleteFileSetStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for DeleteFileSetStep {
    fn name(&self) -> &'static str {
        "delete_file_set"
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!("Deleting file set with id {}", context.file_set_id);

        let res = context
            .repository_manager
            .get_file_set_repository()
            .delete_file_set(context.file_set_id)
            .await;

        match res {
            Ok(_) => {
                if context.deletion_results.is_empty() {
                    tracing::info!(
                        "Deleted file set {} from database. No files associated with file set, skipping remaining steps",
                        context.file_set_id
                    );
                    // No files to process, can skip remaining steps
                    StepAction::Skip
                } else {
                    tracing::info!(
                        "Deleted file set {} from database, proceeding with file deletions",
                        context.file_set_id
                    );
                    StepAction::Continue
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to delete file set {} from database: {}",
                    context.file_set_id,
                    e
                );
                StepAction::Abort(Error::DbError(format!("Failed to delete file set: {}", e)))
            }
        }
    }
}

/// Step 5: Mark files for cloud deletion (if synced)
/// We don't delete from cloud here, just mark them for deletion in the sync logs.
/// The reason is the cloud deletion needs an internet connection but file set deletion should work
/// offline.
pub struct MarkForCloudDeletionStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for MarkForCloudDeletionStep {
    fn name(&self) -> &'static str {
        "mark_for_cloud_deletion"
    }

    fn should_execute(&self, context: &DeletionContext) -> bool {
        // Only execute if there are files to process
        context.deletion_results.values().any(|r| r.is_deletable)
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Marking files for cloud deletion for file set with id {}",
            context.file_set_id
        );
        for deletion_result in context.deletion_results.values_mut() {
            let sync_logs_res = context
                .repository_manager
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

                        let update_res = context
                            .repository_manager
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
                        deletion_result.cloud_sync_marked = true;
                    }
                }
            }
        }

        StepAction::Continue
    }
}

/// Step 6: Delete local files and track results
pub struct DeleteLocalFilesStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for DeleteLocalFilesStep {
    fn name(&self) -> &'static str {
        "delete_local_files"
    }

    fn should_execute(&self, context: &DeletionContext) -> bool {
        context.deletion_results.values().any(|v| v.is_deletable)
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Deleting local files for file set with id {}",
            context.file_set_id
        );

        for deletion_result in context.deletion_results.values_mut() {
            tracing::info!(
                "Processing file info with id {} for local deletion",
                deletion_result.file_info.id
            );
            let file_path = context.settings.get_file_path(
                &deletion_result.file_info.file_type,
                &deletion_result.file_info.archive_file_name,
            );

            let path_str = file_path.to_string_lossy().to_string();
            tracing::info!(
                "Resolved file path for file info id {}: {}",
                deletion_result.file_info.id,
                path_str
            );
            deletion_result.file_path = Some(path_str.clone());

            tracing::info!("Attempting to delete local file: {}", path_str);

            if context.fs_ops.exists(&file_path) {
                tracing::info!("File exists, proceeding with deletion: {}", path_str);
                match context.fs_ops.remove_file(&file_path) {
                    Ok(_) => {
                        tracing::info!("Deleted local file: {}", path_str);
                        deletion_result.file_deletion_success = true;
                    }
                    Err(e) => {
                        tracing::error!("Failed to delete local file {}: {}", path_str, e);
                        deletion_result.file_deletion_success = false;
                        deletion_result.error_messages.push(e.to_string());
                    }
                }
            } else {
                tracing::info!("File {} does not exist, skipping deletion.", path_str);
                deletion_result.file_deletion_success = true; // consider non-existing file as "deleted" (user might have done it manually)
            }
        }

        StepAction::Continue
    }
}

/// Step 7: Delete file_info entries for deleted files from database
pub struct DeleteFileInfosStep;

#[async_trait::async_trait]
impl PipelineStep<DeletionContext> for DeleteFileInfosStep {
    fn name(&self) -> &'static str {
        "delete_file_info_entries"
    }

    fn should_execute(&self, context: &DeletionContext) -> bool {
        context
            .deletion_results
            .values()
            .any(|v| v.is_deletable && v.file_deletion_success)
    }

    async fn execute(&self, context: &mut DeletionContext) -> StepAction {
        tracing::info!(
            "Deleting file_info entries for file set {}",
            context.file_set_id
        );
        for dr in context.deletion_results.values_mut() {
            if !dr.is_deletable || !dr.file_deletion_success {
                tracing::info!(
                    "Skipping file_info with id {} (is_deletable: {}, file_deletion_success: {})",
                    dr.file_info.id,
                    dr.is_deletable,
                    dr.file_deletion_success
                );
                continue;
            }

            tracing::info!(
                "Processing file_info with id {} for deletion",
                dr.file_info.id
            );
            let delete_res = context
                .repository_manager
                .get_file_info_repository()
                .delete_file_info(dr.file_info.id)
                .await;
            match delete_res {
                Ok(_) => {
                    tracing::info!("Deleted file_info with id {} from DB", dr.file_info.id);
                    dr.was_deleted_from_db = true;
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to delete file_info with id {} from DB: {}",
                        dr.file_info.id,
                        e
                    );
                    dr.was_deleted_from_db = false;
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

    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use super::*;
    use crate::{file_system_ops::mock::MockFileSystemOps, view_models::Settings};

    #[async_std::test]
    async fn test_validate_not_in_use_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file2 = ImportedFile {
            original_file_name: "file2.zst".to_string(),
            archive_file_name: "file2.zst".to_string(),
            sha1_checksum: Sha1Checksum::from([1; 20]),
            file_size: 5678,
        };

        let file_set_id =
            prepare_file_set_with_files(&repo_manager, system_id, &[file1, file2]).await;

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
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::new(),
        };

        let step = ValidateNotInUseStep;

        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Abort(_)));

        // Delete release - link to file set should have been deleted also and file set can be
        // deleted now
        repo_manager
            .get_release_repository()
            .delete_release(release_id)
            .await
            .unwrap();

        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
    }

    #[async_std::test]
    async fn test_fetch_file_infos_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

        let file_set_id = prepare_file_set_with_files(&repo_manager, system_id, &[file1]).await;

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::new(),
        };
        let step = FetchFileInfosStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.deletion_results.len(), 1);
        let deletion_result = context.deletion_results.values().next().unwrap();
        assert_eq!(deletion_result.file_info.archive_file_name, "file1.zst");
        // at this point we don't know if the file is deletable yet
        assert!(!deletion_result.is_deletable);
    }

    #[async_std::test]
    async fn test_filter_deletable_files_step() {
        let TestSetup {
            settings,
            repo_manager,
            fs_ops,
            system_id,
            file1,
        } = prepare_test().await;

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

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
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
        };

        // Fetch file infos to populate context
        let fetch_step = FetchFileInfosStep;
        fetch_step.execute(&mut context).await;

        let filter_step = FilterDeletableFilesStep;
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

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = MarkForCloudDeletionStep;
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
        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = DeleteLocalFilesStep;
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
        fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;
        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = DeleteLocalFilesStep;
        let res = step.execute(&mut context).await;

        assert!(
            !context
                .deletion_results
                .get(&file_info.sha1_checksum)
                .unwrap()
                .file_deletion_success
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
        fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut file_deletion_result = FileDeletionResult::new(file_info.clone());
        file_deletion_result.is_deletable = true;

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };
        let step = DeleteLocalFilesStep;
        let res = step.execute(&mut context).await;
        assert!(
            fs_ops.was_deleted(
                settings
                    .get_file_path(&file_info.file_type, &file_info.archive_file_name)
                    .to_string_lossy()
                    .as_ref()
            )
        );

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
        file_deletion_result.file_deletion_success = true;
        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };

        let delete_file_set_step = DeleteFileSetStep;
        delete_file_set_step.execute(&mut context).await;

        let step = DeleteFileInfosStep;
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);

        let deletion_result = context
            .deletion_results
            .get(&file_info.sha1_checksum)
            .unwrap();

        assert!(deletion_result.was_deleted_from_db);

        let res = repo_manager
            .get_file_info_repository()
            .get_file_info(file_info.id)
            .await;
        assert!(res.is_err());
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
        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                file_deletion_result,
            )]),
        };

        let delete_file_set_step = DeleteFileSetStep;
        delete_file_set_step.execute(&mut context).await;

        let step = DeleteFileInfosStep;
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);

        let deletion_result = context
            .deletion_results
            .get(&file_info.sha1_checksum)
            .unwrap();

        assert!(!deletion_result.was_deleted_from_db);

        let res = repo_manager
            .get_file_info_repository()
            .get_file_info(file_info.id)
            .await;
        assert!(res.is_ok());
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
