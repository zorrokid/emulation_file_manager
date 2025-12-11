
use core_types::{events::SyncEvent, FileSyncStatus};

use crate::{
    cloud_sync::context::{FileSyncResult, SyncContext},
    error::Error, pipeline::pipeline_step::{StepAction, PipelineStep},
};

/// Step 1: Prepare files for upload. This involves marking files as pending upload in the
/// database and collecting total number of files to be uploaded.
pub struct PrepareFilesForUploadStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for PrepareFilesForUploadStep {
    fn name(&self) -> &'static str {
        "prepare_files"
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Preparing files for upload");
        let mut offset = 0;
        loop {
            let file_infos_res = context
                .repository_manager
                .get_file_info_repository()
                .get_file_infos_without_sync_log(100, offset)
                .await;

            match file_infos_res {
                Ok(file_infos) => {
                    if file_infos.is_empty() {
                        break;
                    }
                    offset += file_infos.len() as i64;
                    for file_info in &file_infos {
                        let cloud_key = file_info.generate_cloud_key();
                        tracing::debug!(
                            file_info_id = file_info.id,
                            cloud_key = %cloud_key,
                            "Preparing file for sync"
                        );
                       let update_res = context
                            .repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file_info.id,
                                FileSyncStatus::UploadPending,
                                "",
                                cloud_key.as_str(),
                            )
                            .await;
                        if let Err(e) = update_res {
                            tracing::error!(
                                file_info_id = file_info.id,
                                error = %e,
                                "Error updating sync log for file"
                            );
                            return StepAction::Abort(Error::DbError(e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(error = %e, 
                        "Error fetching file infos");
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
            }
        }

        tracing::debug!("File preparation completed");
        StepAction::Continue
    }
}


/// Step 2: Get counts of files prepared for upload and deletion
pub struct GetSyncFileCountsStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for GetSyncFileCountsStep {
    fn name(&self) -> &'static str {
        "get_sync_file_counts"
    }
    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Getting sync file counts");

        let files_to_delete_res = context
            .repository_manager
            .get_file_sync_log_repository()
            .count_logs_by_latest_statuses(&[FileSyncStatus::DeletionPending, FileSyncStatus::DeletionFailed])
            .await;

        match files_to_delete_res {
            Ok(count) => {
                context.files_prepared_for_deletion = count;
                tracing::debug!(count, 
                    "Files prepared for deletion");
            }
            Err(e) => {
                tracing::error!(error = %e, 
                    "Error counting files for deletion");
                return StepAction::Abort(Error::DbError(e.to_string()));
            }
        }

        let files_pending_upload_res = context
            .repository_manager
            .get_file_sync_log_repository()
            .count_logs_by_latest_statuses(&[FileSyncStatus::UploadPending, FileSyncStatus::UploadFailed])
            .await;
        match files_pending_upload_res {
            Ok(count) => {
                context.files_prepared_for_upload = count;
                tracing::debug!(count, 
                    "Files prepared for upload");
            }
            Err(e) => {
                tracing::error!(error = %e, 
                    "Error counting files for upload");
                return StepAction::Abort(Error::DbError(e.to_string()));
            }
        }

        tracing::debug!("File counts retrieval completed");
        StepAction::Continue
    }
}

/// Step 4: Upload pending files to cloud storage
pub struct UploadPendingFilesStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for UploadPendingFilesStep {
    fn name(&self) -> &'static str {
        "upload_pending_files"
    }

    fn should_execute(&self, context: &SyncContext) -> bool {
        context.cloud_ops.is_some() && context.files_prepared_for_upload > 0
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Uploading pending files to cloud storage");
        let mut file_count = 0;

        context
            .progress_tx
            .send(SyncEvent::SyncStarted {
                total_files_count: context.files_prepared_for_upload,
            })
            .await
            .ok(); // TODO: add error handling

        let mut offset = 0;

        loop {
            let pending_files_to_upload_result = context
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_and_file_info_by_sync_status(
                    &[FileSyncStatus::UploadPending, FileSyncStatus::UploadFailed],
                    10,
                    offset,
                )
                .await;

            match pending_files_to_upload_result {
                Err(e) => {
                    tracing::error!(error = %e, 
                        "Error fetching pending files for upload");
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(pending_files) => {
                    tracing::debug!(
                        pending_file_count = pending_files.len(),
                        offset,
                        "Found pending files for upload"
                    );
                    if pending_files.is_empty() {
                        break;
                    }

                    offset += pending_files.len() as u32;

                    for file in pending_files {
                        // check for cancellation
                        if context.cancel_rx.try_recv().is_ok() {
                            tracing::info!("Cloud sync cancelled by user");
                            context.progress_tx
                                .send(SyncEvent::SyncCancelled {})
                                .await
                                .ok(); // TODO: add error handling
                            return StepAction::Abort(Error::OperationCancelled);
                        }

                        tracing::debug!(
                            file_info_id = file.file_info_id,
                            cloud_key = %file.cloud_key,
                            "Uploading file"
                        );
                        file_count += 1;

                        // TODO: maybe trigger only these events for progress tracking
                        context
                            .progress_tx
                            .send(SyncEvent::FileUploadStarted {
                                key: file.cloud_key.clone(),
                                file_number: file_count,
                                total_files: context.files_prepared_for_upload,
                            })
                            .await
                            .ok();

                        let mut file_sync_result = FileSyncResult {
                            file_info_id: file.file_info_id,
                            cloud_key: file.cloud_key.clone(),
                            cloud_operation_success: false,
                            cloud_error: None,
                            db_update_success: false,
                            db_error: None,
                        };

                        let update_res = context
                            .repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file.file_info_id,
                                FileSyncStatus::UploadInProgress,
                                "",
                                &file.cloud_key,
                            )
                            .await;

                        if let Err(e) = update_res {
                            tracing::error!(
                                file_info_id = file.file_info_id,
                                error = %e,
                                "Error updating sync log for file"
                            );
                            file_sync_result.db_error = Some(format!("{}", e));
                            context
                                .upload_results
                                .insert(file.cloud_key.clone(), file_sync_result);
                            context
                                .progress_tx
                                .send(SyncEvent::FileUploadFailed {
                                    key: file.cloud_key.clone(),
                                    error: format!("{}", e),
                                    file_number: file_count,
                                    total_files: context.files_prepared_for_upload,
                                })
                                .await
                                .ok(); // TODO: add error handling

                            // Skip this file and continue with the next one, since status update
                            // failed this will be retried in the next sync run
                            continue;
                        }

                        let local_path = context
                            .settings
                            .get_file_path(&file.file_type, &file.archive_file_name);

                        tracing::debug!(
                            file_info_id = file.file_info_id,
                            local_path = %local_path.display(),
                            "Uploading file to cloud"
                        );

                        let upload_res = context
                            .cloud_ops
                            .as_ref()
                            .unwrap()
                            .upload_file(
                                local_path.as_path(),
                                &file.cloud_key,
                                Some(&context.progress_tx),
                            )
                            .await;

                        match upload_res {
                            Ok(_) => {
                                tracing::info!(
                                    file_info_id = file.file_info_id,
                                    cloud_key = %file.cloud_key,
                                    "Upload succeeded"
                                );
                               file_sync_result.cloud_operation_success = true;

                                let update_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.file_info_id,
                                        FileSyncStatus::UploadCompleted,
                                        "",
                                        &file.cloud_key,
                                    )
                                    .await;

                                match update_res {
                                    Ok(_) => {
                                        file_sync_result.db_update_success = true;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            file_info_id = file.file_info_id,
                                            error = %e,
                                            "Error updating sync log after upload"
                                        );
                                        file_sync_result.db_update_success = false;
                                        file_sync_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                context
                                    .progress_tx
                                    .send(SyncEvent::FileUploadCompleted {
                                        key: file.cloud_key.clone(),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_upload,
                                    })
                                    .await
                                    .ok(); // TODO: add error handling
                            }
                            Err(e) => {
                                tracing::error!(
                                    file_info_id = file.file_info_id,
                                    error = %e,
                                    "Upload failed"
                                );
                                file_sync_result.cloud_operation_success = false;
                                file_sync_result.cloud_error = Some(format!("{}", e));

                                let update_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.file_info_id,
                                        FileSyncStatus::UploadFailed,
                                        &format!("{}", e),
                                        &file.cloud_key,
                                    )
                                    .await;

                                match update_res {
                                    Ok(_) => {
                                        file_sync_result.db_update_success = true;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            file_info_id = file.file_info_id,
                                            error = %e,
                                            "Error updating sync log after failed upload"
                                        );
                                        file_sync_result.db_update_success = false;
                                        file_sync_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                context
                                    .progress_tx
                                    .send(SyncEvent::FileUploadFailed {
                                        key: file.cloud_key.clone(),
                                        error: format!("{}", e),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_upload,
                                    })
                                    .await
                                    .ok(); // TODO: add error handling
                            }
                        }
                        context
                            .upload_results
                            .insert(file.cloud_key.clone(), file_sync_result);
                    }
                }
            }
        }
        context
            .progress_tx
            .send(SyncEvent::SyncCompleted {})
            .await
            .ok(); // TODO: add error handling
        tracing::debug!("Pending file uploads completed");
        StepAction::Continue
    }
}

/// Step 5: Delete the files marked for deletion from cloud storage
pub struct DeleteMarkedFilesStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for DeleteMarkedFilesStep {
    fn name(&self) -> &'static str {
        "delete_marked_files"
    }
    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Deleting marked files from cloud storage");
        let mut file_count = 0;
        let mut offset = 0;

        loop {
            let pending_files_res = context
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_and_file_info_by_sync_status(
                    &[
                        FileSyncStatus::DeletionPending,
                        FileSyncStatus::DeletionFailed,
                    ],
                    10,
                    offset,
                )
                .await;

            match pending_files_res {
                Err(e) => {
                    tracing::error!(error = %e, 
                        "Error fetching pending files for deletion");
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(pending_files) => {
                    if pending_files.is_empty() {
                        break;
                    }

                    tracing::debug!(
                        pending_file_count = pending_files.len(),
                        offset,
                        "Found pending files for deletion"
                    );

                    offset += pending_files.len() as u32;

                    for file in pending_files {
                        // check for cancellation
                        if context.cancel_rx.try_recv().is_ok() {
                            tracing::info!("Cloud sync cancelled by user");
                            context.progress_tx
                                .send(SyncEvent::SyncCancelled {})
                                .await
                                .ok(); // TODO: add error handling

                            return StepAction::Abort(Error::OperationCancelled);
                        }

                        context
                            .progress_tx
                            .send(SyncEvent::FileDeletionStarted {
                                key: file.cloud_key.clone(),
                                file_number: file_count,
                                total_files: context.files_prepared_for_deletion,
                            })
                            .await
                            .ok(); // TODO: add error handling

                        let mut file_deletion_result = FileSyncResult {
                            file_info_id: file.id,
                            cloud_key: file.cloud_key.clone(),
                            cloud_operation_success: false,
                            cloud_error: None,
                            db_update_success: false,
                            db_error: None,
                        };

                        let update_res = context
                            .repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file.id,
                                FileSyncStatus::DeletionInProgress,
                                "",
                                &file.cloud_key,
                            )
                            .await
                            .map_err(|e| Error::DbError(e.to_string()));

                        if let Err(e) = update_res {
                            tracing::error!(
                                file_info_id = file.id,
                                error = %e,
                                "Error updating sync log for file deletion"
                            );
                            file_deletion_result.db_error = Some(format!("{}", e));
                            context
                                .deletion_results
                                .insert(file.cloud_key.clone(), file_deletion_result);
                            context
                                .progress_tx
                                .send(SyncEvent::FileDeletionFailed {
                                    key: file.cloud_key.clone(),
                                    error: format!("{}", e),
                                    file_number: file_count,
                                    total_files: context.files_prepared_for_deletion,
                                })
                                .await
                                .ok(); // TODO: add error handling

                            // Skip this file and continue with the next one, since status update
                            // failed this will be retried in the next sync run
                            continue;
                        }

                        file_count += 1;
                        let deletion_res = context
                            .cloud_ops
                            .as_ref()
                            .unwrap()
                            .delete_file(&file.cloud_key)
                            .await;

                        match deletion_res {
                            Ok(_) => {
                                file_deletion_result.cloud_operation_success = true;
                                let update_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::DeletionCompleted,
                                        "",
                                        &file.cloud_key,
                                    )
                                    .await;

                                match update_res {
                                    Ok(_) => {
                                        file_deletion_result.db_update_success = true;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            file_info_id = file.id,
                                            error = %e,
                                            "Error updating sync log after deletion"
                                        );
                                        file_deletion_result.db_update_success = false;
                                        file_deletion_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                context
                                    .progress_tx
                                    .send(SyncEvent::FileDeletionCompleted {
                                        key: file.cloud_key.clone(),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_deletion,
                                    })
                                    .await
                                    .ok(); // TODO: add error handling
                            }
                            Err(e) => {
                                tracing::error!(
                                    file_info_id = file.id,
                                    error = %e,
                                    "File deletion failed"
                                );
                                file_deletion_result.cloud_operation_success = false;
                                file_deletion_result.cloud_error = Some(format!("{}", e));
                                let update_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::DeletionFailed,
                                        &format!("{}", e),
                                        &file.cloud_key,
                                    )
                                    .await;

                                match update_res {
                                    Ok(_) => {
                                        file_deletion_result.db_update_success = true;
                                    }
                                    Err(e) => {
                                        file_deletion_result.db_update_success = false;
                                        file_deletion_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                context
                                    .progress_tx
                                    .send(SyncEvent::FileDeletionFailed {
                                        key: file.cloud_key.clone(),
                                        error: format!("{}", e),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_deletion,
                                    })
                                    .await
                                    .ok(); //  TODO: add error handling
                            }
                        }
                        context
                            .deletion_results
                            .insert(file.cloud_key.clone(), file_deletion_result);
                    }
                }
            }
        }

        tracing::debug!("Marked file deletions completed");
        StepAction::Continue
    }
}

/// Step 6: Clean up sync log entries for deleted file_info records
pub struct CleanupOrphanedSyncLogsStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for CleanupOrphanedSyncLogsStep {
    fn name(&self) -> &'static str {
        "cleanup_orphaned_sync_logs"
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Cleaning up orphaned sync log entries");

        let cleanup_res = context
            .repository_manager
            .get_file_sync_log_repository()
            .cleanup_orphaned_logs()
            .await;

        match cleanup_res {
            Ok(rows_deleted) => {
                if rows_deleted > 0 {
                    tracing::info!(
                        "Cleaned up {} orphaned sync log entries",
                        rows_deleted
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to cleanup orphaned sync log entries"
                );
                // Don't abort - this is a cleanup operation
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use cloud_storage::{mock::MockCloudStorage};
    use core_types::{events::SyncEvent, FileSyncStatus, FileType, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        cloud_sync::{
            context::SyncContext,
            steps::{
                GetSyncFileCountsStep, PrepareFilesForUploadStep, UploadPendingFilesStep,
            },
        }, pipeline::pipeline_step::{StepAction, PipelineStep}, settings_service::SettingsService, view_models::Settings
    };

    #[async_std::test]
    async fn test_prepare_files_for_upload_step() {
        let mut context = initialize_sync_context().await;

        let _file_info_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                "file1.zst",
                FileType::Rom,
            )
            .await
            .unwrap();

        let pending_files_to_upload_result = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_logs_and_file_info_by_sync_status(&[FileSyncStatus::UploadPending], 10, 0)
            .await
            .unwrap();

        assert_eq!(pending_files_to_upload_result.len(), 0);

        let file_infos_res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_without_sync_log(100, 0)
            .await
            .unwrap();

        assert_eq!(file_infos_res.len(), 1);
        assert_eq!(file_infos_res[0].archive_file_name, "file1.zst");

        let step = PrepareFilesForUploadStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);

        let file_infos_res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_without_sync_log(100, 0)
            .await
            .unwrap();

        let pending_files_to_upload_result = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_logs_and_file_info_by_sync_status(&[FileSyncStatus::UploadPending], 10, 0)
            .await
            .unwrap();

        assert_eq!(
            pending_files_to_upload_result[0].archive_file_name,
            "file1.zst"
        );

        assert_eq!(file_infos_res.len(), 0);
    }

    #[async_std::test]
    async fn test_get_sync_file_counts_step() {
        let mut context = initialize_sync_context().await;

        let file_info_id_1 = add_file_info(&context.repository_manager, [0; 20], "file1.zst", FileType::Rom).await;
        let file_info_id_2 = add_file_info(&context.repository_manager, [1; 20], "file2.zst", FileType::Rom).await;
        let file_info_id_3 = add_file_info(&context.repository_manager, [2; 20], "file3.zst", FileType::Rom).await;
        let file_info_id_4 = add_file_info(&context.repository_manager, [3; 20], "file4.zst", FileType::Rom).await;
        let file_info_id_5 = add_file_info(&context.repository_manager, [4; 20], "file5.zst", FileType::Rom).await;

        add_log_entry(&context.repository_manager, file_info_id_1, FileSyncStatus::UploadPending, "rom/file1.zst").await;
        add_log_entry(&context.repository_manager, file_info_id_2, FileSyncStatus::UploadFailed, "rom/file2.zst").await;
        add_log_entry(&context.repository_manager, file_info_id_3, FileSyncStatus::DeletionPending, "rom/file3.zst").await;
        add_log_entry(&context.repository_manager, file_info_id_4, FileSyncStatus::DeletionFailed, "rom/file4.zst").await;
        add_log_entry(&context.repository_manager, file_info_id_5, FileSyncStatus::UploadPending, "rom/file5.zst").await;
        add_log_entry(&context.repository_manager, file_info_id_5, FileSyncStatus::UploadCompleted, "rom/file5.zst").await;
        
        let step = GetSyncFileCountsStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);
        assert_eq!(context.files_prepared_for_deletion, 2);
        assert_eq!(context.files_prepared_for_upload, 2);
    }

    async fn add_file_info(repo_manager: &RepositoryManager, checksum: [u8; 20], file_name: &str, file_type: FileType) -> i64 {
        repo_manager
            .get_file_info_repository()
            .add_file_info(&Sha1Checksum::from(checksum), 1234, file_name, file_type)
            .await
            .unwrap()
    }

    async fn add_log_entry(repo_manager: &RepositoryManager, file_info_id: i64, status: FileSyncStatus, cloud_key: &str) {
        repo_manager
            .get_file_sync_log_repository()
            .add_log_entry(file_info_id, status, "", cloud_key)
            .await
            .unwrap();
    }

    #[async_std::test]
    async fn test_upload_pending_files_step() {
        let mut context = initialize_sync_context().await;

        let file_info_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                "file1.zst",
                FileType::Rom,
            )
            .await
            .unwrap();

        context
            .repository_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::UploadPending,
                "",
                "rom/file1.zst",
            )
            .await
            .unwrap();

        context.files_prepared_for_upload = 1;

        let step = crate::cloud_sync::steps::UploadPendingFilesStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);

        let upload_result = context.upload_results.get("rom/file1.zst").unwrap();

        assert!(upload_result.cloud_operation_success);
        assert!(upload_result.db_update_success);

        let log_entry = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info_id)
            .await
            .unwrap();

        assert_eq!(
            log_entry.first().unwrap().status,
            FileSyncStatus::UploadCompleted
        );
    }

    #[async_std::test]
    async fn test_delete_marked_files_step() {
        let mut context = initialize_sync_context().await;
        let file_info_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                "file1.zst",
                FileType::Rom,
            )
            .await
            .unwrap();
        context
            .repository_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info_id,
                FileSyncStatus::DeletionPending,
                "",
                "rom/file1.zst",
            )
            .await
            .unwrap();

        context.files_prepared_for_deletion = 1;
        let step = crate::cloud_sync::steps::DeleteMarkedFilesStep;
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);
        let deletion_result = context.deletion_results.get("rom/file1.zst").unwrap();
        assert!(deletion_result.cloud_operation_success);
        assert!(deletion_result.db_update_success);
        let log_entry = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(
            log_entry.first().unwrap().status,
            FileSyncStatus::DeletionCompleted
        );
    }

    async fn initialize_sync_context() -> SyncContext {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });

        let settings_service = Arc::new(SettingsService::new(repo_manager.clone())); 
        let cloud_ops = Arc::new(MockCloudStorage::new());

        let (tx, _rx) = async_std::channel::unbounded();
        let (_cancel_tx, cancel_rx) = async_std::channel::unbounded::<()>();

        SyncContext {
            settings,
            repository_manager: repo_manager,
            cloud_ops: Some(cloud_ops),
            progress_tx: tx,
            files_prepared_for_upload: 0,
            files_prepared_for_deletion: 0,
            upload_results: HashMap::new(),
            deletion_results: HashMap::new(),
            settings_service,
            cancel_rx,
        }
    }

     #[async_std::test]
     async fn test_upload_progress_messages() {
         let pool = Arc::new(setup_test_db().await);
         let repo_manager = Arc::new(RepositoryManager::new(pool));
         let settings = Arc::new(Settings {
             collection_root_dir: PathBuf::from("/"),
             ..Default::default()
         });
         let cloud_ops = Arc::new(MockCloudStorage::new());
         
         let (tx, rx) = async_std::channel::unbounded();
         let settings_service = Arc::new(SettingsService::new(repo_manager.clone())); 
         let (_cancel_tx, cancel_rx) = async_std::channel::unbounded::<()>();
         
         let mut context = SyncContext {
             settings,
             repository_manager: repo_manager,
             cloud_ops: Some(cloud_ops),
             progress_tx: tx,
             files_prepared_for_upload: 0,
             files_prepared_for_deletion: 0,
             upload_results: HashMap::new(),
             deletion_results: HashMap::new(),
                settings_service,
                cancel_rx,
         };
     
         let file_info_id = context
             .repository_manager
             .get_file_info_repository()
             .add_file_info(&Sha1Checksum::from([0; 20]), 1234, "file1.zst", FileType::Rom)
             .await
             .unwrap();
     
         context
             .repository_manager
             .get_file_sync_log_repository()
             .add_log_entry(file_info_id, FileSyncStatus::UploadPending, "", "rom/file1.zst")
             .await
             .unwrap();
     
         context.files_prepared_for_upload = 1;
     
         // Execute step
         let step = UploadPendingFilesStep;
         step.execute(&mut context).await;
     
         // Collect all messages from receiver
         let mut messages = Vec::new();
         while let Ok(msg) = rx.try_recv() {
             messages.push(msg);
         }
     
         // Assert expected messages
         assert_eq!(messages.len(), 7);

          assert!(matches!(messages[0], SyncEvent::SyncStarted { 
             total_files_count: 1 
         }));
        
         assert!(matches!(messages[1], SyncEvent::FileUploadStarted { 
             ref key, file_number: 1, total_files: 1 
         } if key == "rom/file1.zst"));

        // by default the mock simulates uploading in 3 parts 
        assert!(matches!(messages[2], SyncEvent::PartUploaded { 
             ref key, part: 1 
         } if key == "rom/file1.zst"));

        assert!(matches!(messages[3], SyncEvent::PartUploaded { 
             ref key, part: 2 
         } if key == "rom/file1.zst"));

        assert!(matches!(messages[4], SyncEvent::PartUploaded { 
             ref key, part: 3 
         } if key == "rom/file1.zst"));

         assert!(matches!(messages[5], SyncEvent::FileUploadCompleted{ 
             ref key, file_number: 1, total_files: 1 
         } if key == "rom/file1.zst"));

        assert!(matches!(messages[6], SyncEvent::SyncCompleted {}));
     }
}
