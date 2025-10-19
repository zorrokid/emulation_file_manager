use core_types::FileSyncStatus;

use crate::{
    error::Error,
    file_set_deletion::context::{DeletionContext, FileDeletionResult},
    file_system_ops::FileSystemOps,
};

/// Step 1: Validate that the file set is not in use by any releases
pub struct ValidateNotInUseStep;

/// Result of executing a pipeline step
#[derive(Debug, Clone, PartialEq)]
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
    fn name(&self) -> &'static str;

    /// Determines if this step should execute based on current context
    fn should_execute(&self, _context: &DeletionContext<F>) -> bool {
        true // By default, always execute
    }

    /// Execute the step, modifying the context and returning the next action
    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction;
}

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for ValidateNotInUseStep {
    fn name(&self) -> &'static str {
        "validate_not_in_use"
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        let is_in_use_res = context
            .repository_manager
            .get_file_set_repository()
            .is_in_use(context.file_set_id)
            .await;
        match is_in_use_res {
            Err(e) => StepAction::Abort(Error::DbError(format!(
                "Failed to check if file set is in use: {}",
                e
            ))),
            Ok(in_use) => {
                if in_use {
                    StepAction::Abort(Error::DbError(
                        "File set is in use by one or more releases".to_string(),
                    ))
                } else {
                    StepAction::Continue
                }
            }
        }
    }
}

/// Step 2: Fetch all file infos for the file set
pub struct FetchFileInfosStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for FetchFileInfosStep {
    fn name(&self) -> &'static str {
        "fetch_file_infos"
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        let file_infos_res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(context.file_set_id)
            .await;

        match file_infos_res {
            Ok(file_infos) => {
                // even if file_infos is empty, continue to delete the file set
                context.deletion_results = file_infos
                    .into_iter()
                    .map(|fi| {
                        (
                            fi.sha1_checksum.clone(),
                            FileDeletionResult {
                                file_info: fi,
                                file_path: None,
                                file_deletion_success: false,
                                error_messages: vec![],
                                is_deletable: false,
                                was_deleted_from_db: false,
                                cloud_sync_marked: false,
                            },
                        )
                    })
                    .collect();

                StepAction::Continue
            }
            Err(e) => {
                StepAction::Abort(Error::DbError(format!("Failed to fetch file infos: {}", e)))
            }
        }
    }
}

/// Step 3: Delete the file set from database
pub struct DeleteFileSetStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for DeleteFileSetStep {
    fn name(&self) -> &'static str {
        "delete_file_set"
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        println!("Deleting file set {}", context.file_set_id);
        let res = context
            .repository_manager
            .get_file_set_repository()
            .delete_file_set(context.file_set_id)
            .await;

        println!("Deleted file set {}: {:?}", context.file_set_id, res);
        match res {
            Ok(_) => {
                if context.deletion_results.is_empty() {
                    // No files to process, can skip remaining steps
                    StepAction::Skip
                } else {
                    StepAction::Continue
                }
            }
            Err(e) => {
                StepAction::Abort(Error::DbError(format!("Failed to delete file set: {}", e)))
            }
        }
    }
}

/// Step 4: Filter files that are only in this file set (safe to delete)
pub struct FilterDeletableFilesStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for FilterDeletableFilesStep {
    fn name(&self) -> &'static str {
        "filter_deletable_files"
    }

    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        // Only execute if there are files to process
        !context.deletion_results.is_empty()
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        for deletion_result in context.deletion_results.values_mut() {
            let file_sets_res = context
                .repository_manager
                .get_file_set_repository()
                .get_file_sets_by_file_info(deletion_result.file_info.id)
                .await;

            match file_sets_res {
                Err(e) => {
                    return StepAction::Abort(Error::DbError(format!(
                        "Failed to fetch file sets for file info {}: {}",
                        deletion_result.file_info.id, e
                    )))
                }
                Ok(file_sets) => {
                    // Only delete if file is used in exactly this one file set
                    if let [single_entry] = &file_sets[..] {
                        if single_entry.id == context.file_set_id {
                            deletion_result.is_deletable = true;
                        }
                    }
                }
            }
        }

        StepAction::Continue
    }
}

/// Step 5: Mark files for cloud deletion (if synced)
pub struct MarkForCloudDeletionStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for MarkForCloudDeletionStep {
    fn name(&self) -> &'static str {
        "mark_for_cloud_deletion"
    }

    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        // Only execute if there are files to process
        context.deletion_results.values().any(|r| r.is_deletable)
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        for deletion_result in context.deletion_results.values_mut() {
            let sync_logs_res = context
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_by_file_info(deletion_result.file_info.id)
                .await;

            println!(
                "Marking file info {} for cloud deletion check",
                deletion_result.file_info.id
            );

            match sync_logs_res {
                Err(e) => {
                    return StepAction::Abort(Error::DbError(format!(
                        "Failed to fetch sync logs for file info {}: {}",
                        deletion_result.file_info.id, e
                    )))
                }
                Ok(sync_logs) => {
                    if let Some(entry) = sync_logs.last() {
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
                            return StepAction::Abort(Error::DbError(format!(
                                "Failed to mark file info {} for cloud deletion: {}",
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
impl<F: FileSystemOps> DeletionStep<F> for DeleteLocalFilesStep {
    fn name(&self) -> &'static str {
        "delete_local_files"
    }

    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        context.deletion_results.values().any(|v| v.is_deletable)
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        for deletion_result in context.deletion_results.values_mut() {
            let file_path = context.settings.get_file_path(
                &deletion_result.file_info.file_type,
                &deletion_result.file_info.archive_file_name,
            );

            let path_str = file_path.to_string_lossy().to_string();
            deletion_result.file_path = Some(path_str.clone());

            println!("Deleting local file: {}", path_str);
            if context.fs_ops.exists(&file_path) {
                println!("File exists, attempting deletion: {}", path_str);
                match context.fs_ops.remove_file(&file_path) {
                    Ok(_) => {
                        println!("Deleted local file: {}", path_str);
                        deletion_result.file_deletion_success = true;
                    }
                    Err(e) => {
                        println!("Failed to delete local file {}: {}", path_str, e);
                        deletion_result.file_deletion_success = false;
                        deletion_result.error_messages.push(e.to_string());
                    }
                }
            } else {
                println!("File does not exist, skipping deletion: {}", path_str);
                deletion_result.file_deletion_success = true; // consider non-existing file as "deleted" (user might have done it manually)
            }
        }

        StepAction::Continue
    }
}

/// Step 7: Delete file_info entries for deleted files from database
pub struct DeleteFileInfosStep;

#[async_trait::async_trait]
impl<F: FileSystemOps> DeletionStep<F> for DeleteFileInfosStep {
    fn name(&self) -> &'static str {
        "delete_file_info_entries"
    }

    fn should_execute(&self, context: &DeletionContext<F>) -> bool {
        context
            .deletion_results
            .values()
            .any(|v| v.is_deletable && v.file_deletion_success)
    }

    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction {
        for dr in context.deletion_results.values_mut() {
            println!(
                "Deleting file_info {} from DB",
                dr.file_info.archive_file_name
            );
            let delete_res = context
                .repository_manager
                .get_file_info_repository()
                .delete_file_info(dr.file_info.id)
                .await;
            println!(
                "Delete file_info {} result: {:?}",
                dr.file_info.archive_file_name, delete_res
            );

            match delete_res {
                Ok(_) => {
                    dr.was_deleted_from_db = true;
                }
                Err(e) => {
                    dr.was_deleted_from_db = false;
                    dr.error_messages
                        .push(format!("Failed to delete file_info from DB: {}", e));
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

        let file_set_id = repo_manager
            .get_file_set_repository()
            .add_file_set(
                "test_set",
                "file name",
                &FileType::Rom,
                "",
                &[file1, file2],
                &[system_id],
            )
            .await
            .unwrap();

        // add another file set that uses file2
        let _another_file_set_id = repo_manager
            .get_file_set_repository()
            .add_file_set(
                "another_set",
                "file name",
                &FileType::Rom,
                "",
                &[file2_clone],
                &[system_id],
            )
            .await
            .unwrap();

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
                    FileDeletionResult {
                        file_info: file_info_1.clone(),
                        file_path: None,
                        file_deletion_success: false,
                        error_messages: vec![],
                        is_deletable: false,
                        was_deleted_from_db: false,
                        cloud_sync_marked: false,
                    },
                ),
                (
                    file_info_2.sha1_checksum.clone(),
                    FileDeletionResult {
                        file_info: file_info_2.clone(),
                        file_path: None,
                        file_deletion_success: false,
                        error_messages: vec![],
                        is_deletable: false,
                        was_deleted_from_db: false,
                        cloud_sync_marked: false,
                    },
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

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                FileDeletionResult {
                    file_info: file_info.clone(),
                    file_path: None,
                    file_deletion_success: false,
                    error_messages: vec![],
                    is_deletable: true,
                    was_deleted_from_db: false,
                    cloud_sync_marked: false,
                },
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

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();
        let file_info = file_infos.first().unwrap();

        // Let's not add the file to fs_ops, simulating that it doesn't exist
        // let file_path = settings.get_file_path(&file_info.file_type, &file_info.archive_file_name);
        // fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                FileDeletionResult {
                    file_info: file_info.clone(),
                    file_path: None,
                    file_deletion_success: false,
                    error_messages: vec![],
                    is_deletable: true,
                    was_deleted_from_db: false,
                    cloud_sync_marked: false,
                },
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

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();
        let file_info = file_infos.first().unwrap();
        let file_path = settings.get_file_path(&file_info.file_type, &file_info.archive_file_name);
        fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                FileDeletionResult {
                    file_info: file_info.clone(),
                    file_path: None,
                    file_deletion_success: false,
                    error_messages: vec![],
                    is_deletable: true,
                    was_deleted_from_db: false,
                    cloud_sync_marked: false,
                },
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

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();
        let file_info = file_infos.first().unwrap();
        let file_path = settings.get_file_path(&file_info.file_type, &file_info.archive_file_name);
        fs_ops.add_file(file_path.to_string_lossy().as_ref());

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                FileDeletionResult {
                    file_info: file_info.clone(),
                    file_path: None,
                    file_deletion_success: false,
                    error_messages: vec![],
                    is_deletable: true,
                    was_deleted_from_db: false,
                    cloud_sync_marked: false,
                },
            )]),
        };
        let step = DeleteLocalFilesStep;
        let res = step.execute(&mut context).await;
        assert!(fs_ops.was_deleted(
            settings
                .get_file_path(&file_info.file_type, &file_info.archive_file_name)
                .to_string_lossy()
                .as_ref()
        ));

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

        let file_infos = repo_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        let file_info = file_infos.first().unwrap();

        let mut context = DeletionContext {
            file_set_id,
            repository_manager: repo_manager.clone(),
            settings: settings.clone(),
            fs_ops: fs_ops.clone(),
            deletion_results: HashMap::from([(
                file_info.sha1_checksum.clone(),
                FileDeletionResult {
                    file_info: file_info.clone(),
                    file_path: None,
                    file_deletion_success: true,
                    error_messages: vec![],
                    is_deletable: true,
                    was_deleted_from_db: false,
                    cloud_sync_marked: false,
                },
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
