use core_types::{CloudSyncStatus, FileSyncStatus, events::SyncEvent};
use flume::Sender;

use crate::{
    cloud_sync::context::{FileSyncResult, SyncContext},
    error::Error,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

// TODO move to utils module?
async fn send_progress_event(event: SyncEvent, progress_tx: &Sender<SyncEvent>) {
    let res = progress_tx.send(event);

    if let Err(e) = res {
        tracing::error!("Sending sync event failed {}", e);
    }
}

/// Step 2: Upload files with `cloud_sync_status = NotSynced` to cloud storage.
/// On success, sets `cloud_sync_status = Synced` and writes an `UploadCompleted` log entry.
/// On failure, leaves `cloud_sync_status = NotSynced` (auto-retried next sync) and writes
/// an `UploadFailed` log entry for diagnosis.
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
        let mut session_skip: i64 = 0;

        loop {
            let pending_files_result = context
                .repository_manager
                .get_file_info_repository()
                .get_files_pending_upload(10, session_skip)
                .await;

            match pending_files_result {
                Err(e) => {
                    tracing::error!(error = %e, "Error fetching pending files for upload");
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(pending_files) => {
                    tracing::debug!(
                        pending_file_count = pending_files.len(),
                        "Found pending files for upload"
                    );
                    if pending_files.is_empty() {
                        break;
                    }

                    for file in pending_files {
                        if context.cancel_rx.try_recv().is_ok() {
                            tracing::info!("Cloud sync cancelled by user");
                            return StepAction::Abort(Error::OperationCancelled);
                        }

                        let cloud_key = cloud_storage::cloud_key(
                            file.file_type,
                            &file.archive_file_name,
                        );

                        let local_path = context
                            .settings
                            .get_file_path(&file.file_type, &file.archive_file_name);

                        tracing::debug!(
                            file_info_id = file.id,
                            cloud_key = %cloud_key,
                            local_path = %local_path.display(),
                            "Uploading file"
                        );

                        file_count += 1;

                        send_progress_event(
                            SyncEvent::FileUploadStarted {
                                key: cloud_key.clone(),
                                file_number: file_count,
                                total_files: context.files_prepared_for_upload,
                            },
                            &context.progress_tx,
                        )
                        .await;

                        let mut file_sync_result = FileSyncResult {
                            file_info_id: file.id,
                            cloud_key: cloud_key.clone(),
                            cloud_operation_success: false,
                            cloud_error: None,
                            db_update_success: false,
                            db_error: None,
                        };

                        let upload_res = context
                            .cloud_ops
                            .as_ref()
                            .expect("cloud_ops guaranteed by should_execute")
                            .upload_file(
                                local_path.as_path(),
                                &cloud_key,
                                Some(&context.progress_tx),
                            )
                            .await;

                        match upload_res {
                            Ok(()) => {
                                tracing::info!(
                                    file_info_id = file.id,
                                    cloud_key = %cloud_key,
                                    "Upload succeeded"
                                );
                                file_sync_result.cloud_operation_success = true;

                                // Update cloud_sync_status to Synced
                                let status_res = context
                                    .repository_manager
                                    .get_file_info_repository()
                                    .update_cloud_sync_status(file.id, CloudSyncStatus::Synced)
                                    .await;

                                // Record audit log entry
                                let log_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::UploadCompleted,
                                        "",
                                        &cloud_key,
                                    )
                                    .await;

                                match (status_res, log_res) {
                                    (Ok(()), Ok(_)) => {
                                        file_sync_result.db_update_success = true;
                                    }
                                    (Err(e), _) => {
                                        tracing::error!(
                                            file_info_id = file.id,
                                            error = %e,
                                            "Error updating cloud_sync_status after upload"
                                        );
                                        file_sync_result.db_update_success = false;
                                        file_sync_result.db_error = Some(format!("{}", e));
                                    }
                                    (_, Err(e)) => {
                                        tracing::error!(
                                            file_info_id = file.id,
                                            error = %e,
                                            "Error writing upload log entry"
                                        );
                                        file_sync_result.db_update_success = false;
                                        file_sync_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                send_progress_event(
                                    SyncEvent::FileUploadCompleted {
                                        key: cloud_key.clone(),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_upload,
                                    },
                                    &context.progress_tx,
                                )
                                .await;
                            }
                            Err(e) => {
                                tracing::error!(
                                    file_info_id = file.id,
                                    cloud_key = cloud_key,
                                    local_path = %local_path.display(),
                                    error = %e,
                                    "Upload failed"
                                );
                                file_sync_result.cloud_operation_success = false;
                                file_sync_result.cloud_error = Some(format!("{}", e));

                                // Leave cloud_sync_status = NotSynced so it is retried next sync.
                                // Write audit log entry for diagnosis.
                                let log_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::UploadFailed,
                                        &format!("{}", e),
                                        &cloud_key,
                                    )
                                    .await;

                                match log_res {
                                    Ok(_) => {
                                        file_sync_result.db_update_success = true;
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            file_info_id = file.id,
                                            error = %e,
                                            "Error writing failure log after failed upload"
                                        );
                                        file_sync_result.db_update_success = false;
                                        file_sync_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                send_progress_event(
                                    SyncEvent::FileUploadFailed {
                                        key: cloud_key.clone(),
                                        error: format!("{e}"),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_upload,
                                    },
                                    &context.progress_tx,
                                )
                                .await;

                                // Failed files stay NotSynced and re-appear at offset 0 on the
                                // next fetch, so advance the session offset past them.
                                session_skip += 1;
                            }
                        }
                        context.upload_results.insert(cloud_key, file_sync_result);
                    }
                }
            }
        }
        tracing::debug!("Pending file uploads completed");
        StepAction::Continue
    }
}

/// Step 3: Delete files with `cloud_sync_status = DeletionPending` AND `archive_file_name IS NOT NULL`
/// from cloud storage, then remove the tombstone `file_info` record.
/// On failure, leaves `cloud_sync_status = DeletionPending` for retry on next sync.
pub struct DeleteCloudFilesStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for DeleteCloudFilesStep {
    fn name(&self) -> &'static str {
        "delete_cloud_files"
    }

    fn should_execute(&self, context: &SyncContext) -> bool {
        context.cloud_ops.is_some() && context.cloud_files_prepared_for_deletion > 0
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Deleting cloud files");
        let mut file_count = 0;
        let mut session_skip: i64 = 0;

        loop {
            let pending_files_res = context
                .repository_manager
                .get_file_info_repository()
                .get_cloud_files_pending_deletion(10, session_skip)
                .await;

            match pending_files_res {
                Err(e) => {
                    tracing::error!(error = %e, "Error fetching pending files for deletion");
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(pending_files) => {
                    if pending_files.is_empty() {
                        break;
                    }

                    tracing::debug!(
                        pending_file_count = pending_files.len(),
                        "Found pending files for deletion"
                    );

                    for file in pending_files {
                        if context.cancel_rx.try_recv().is_ok() {
                            tracing::info!("Cloud sync cancelled by user");
                            return StepAction::Abort(Error::OperationCancelled);
                        }

                        let cloud_key =
                            cloud_storage::cloud_key(file.file_type, &file.archive_file_name);

                        file_count += 1;
                        send_progress_event(
                            SyncEvent::FileDeletionStarted {
                                key: cloud_key.clone(),
                                file_number: file_count,
                                total_files: context.cloud_files_prepared_for_deletion,
                            },
                            &context.progress_tx,
                        )
                        .await;

                        let mut file_deletion_result = FileSyncResult {
                            file_info_id: file.id,
                            cloud_key: cloud_key.clone(),
                            cloud_operation_success: false,
                            cloud_error: None,
                            db_update_success: false,
                            db_error: None,
                        };

                        let deletion_res = context
                            .cloud_ops
                            .as_ref()
                            .expect("cloud_ops guaranteed by should_execute")
                            .delete_file(&cloud_key)
                            .await;

                        match deletion_res {
                            Ok(_) => {
                                file_deletion_result.cloud_operation_success = true;

                                // Write audit log entry (no FK constraint — safe even after
                                // file_info is deleted).
                                let log_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::DeletionCompleted,
                                        "",
                                        &cloud_key,
                                    )
                                    .await;

                                // Cloud deletion succeeded: remove the tombstone file_info record.
                                let delete_res = context
                                    .repository_manager
                                    .get_file_info_repository()
                                    .delete_file_info(file.id)
                                    .await;

                                match (log_res, delete_res) {
                                    (Ok(_), Ok(_)) => {
                                        file_deletion_result.db_update_success = true;
                                        tracing::info!(
                                            file_info_id = file.id,
                                            cloud_key = %cloud_key,
                                            "Cloud deletion succeeded; file_info record removed"
                                        );
                                    }
                                    (Err(e), _) => {
                                        tracing::error!(
                                            file_info_id = file.id,
                                            error = %e,
                                            "Cloud deletion succeeded but could not write audit log"
                                        );
                                        file_deletion_result.db_update_success = false;
                                        file_deletion_result.db_error = Some(format!("{}", e));
                                    }
                                    (_, Err(e)) => {
                                        tracing::error!(
                                            file_info_id = file.id,
                                            error = %e,
                                            "Cloud deletion succeeded but could not remove file_info record"
                                        );
                                        file_deletion_result.db_update_success = false;
                                        file_deletion_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                send_progress_event(
                                    SyncEvent::FileDeletionCompleted {
                                        key: cloud_key.clone(),
                                        file_number: file_count,
                                        total_files: context.cloud_files_prepared_for_deletion,
                                    },
                                    &context.progress_tx,
                                )
                                .await;
                            }
                            Err(e) => {
                                tracing::error!(
                                    file_info_id = file.id,
                                    cloud_key = cloud_key,
                                    error = %e,
                                    "File deletion failed"
                                );
                                file_deletion_result.cloud_operation_success = false;
                                file_deletion_result.cloud_error = Some(format!("{}", e));

                                // Leave cloud_sync_status = DeletionPending for retry.
                                let log_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::DeletionFailed,
                                        &format!("{}", e),
                                        &cloud_key,
                                    )
                                    .await;

                                match log_res {
                                    Ok(_) => {
                                        file_deletion_result.db_update_success = true;
                                    }
                                    Err(e) => {
                                        file_deletion_result.db_update_success = false;
                                        file_deletion_result.db_error = Some(format!("{}", e));
                                    }
                                }

                                send_progress_event(
                                    SyncEvent::FileDeletionFailed {
                                        key: cloud_key.clone(),
                                        error: format!("{e}"),
                                        file_number: file_count,
                                        total_files: context.cloud_files_prepared_for_deletion,
                                    },
                                    &context.progress_tx,
                                )
                                .await;

                                // Failed files stay DeletionPending and re-appear at offset 0 on
                                // the next fetch, so advance the session offset past them.
                                session_skip += 1;
                            }
                        }
                        context
                            .deletion_results
                            .insert(cloud_key, file_deletion_result);
                    }
                }
            }
        }

        tracing::debug!("Cloud file deletions completed");
        StepAction::Continue
    }
}

/// Step 4: Delete `DeletionPending` tombstones with no `archive_file_name` from the database.
/// These records were never uploaded to cloud storage, so only a DB delete is needed.
pub struct CleanupTombstonesStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for CleanupTombstonesStep {
    fn name(&self) -> &'static str {
        "cleanup_tombstones"
    }

    fn should_execute(&self, context: &SyncContext) -> bool {
        context.tombstones_prepared_for_cleanup > 0
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Cleaning up tombstones");

        loop {
            let tombstones_res = context
                .repository_manager
                .get_file_info_repository()
                .get_tombstones_pending_deletion(10, 0)
                .await;

            match tombstones_res {
                Err(e) => {
                    tracing::error!(error = %e, "Error fetching tombstones for cleanup");
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(tombstones) => {
                    if tombstones.is_empty() {
                        break;
                    }

                    let mut batch_cleaned = 0;
                    for tombstone in tombstones {
                        if context.cancel_rx.try_recv().is_ok() {
                            tracing::info!("Cloud sync cancelled by user");
                            return StepAction::Abort(Error::OperationCancelled);
                        }

                        match context
                            .repository_manager
                            .get_file_info_repository()
                            .delete_file_info(tombstone.id)
                            .await
                        {
                            Ok(_) => {
                                tracing::info!(
                                    file_info_id = tombstone.id,
                                    "Tombstone cleaned up"
                                );
                                batch_cleaned += 1;
                                context.tombstones_cleaned_up += 1;
                            }
                            Err(e) => {
                                tracing::error!(
                                    file_info_id = tombstone.id,
                                    error = %e,
                                    "Failed to clean up tombstone"
                                );
                            }
                        }
                    }
                    // Break if no progress was made to prevent an infinite loop when DB
                    // deletes consistently fail within a batch.
                    if batch_cleaned == 0 {
                        break;
                    }
                }
            }
        }

        tracing::debug!("Tombstone cleanup completed");
        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use cloud_storage::mock::MockCloudStorage;
    use core_types::{CloudSyncStatus, FileSyncStatus, FileType, Sha1Checksum, events::SyncEvent};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        cloud_sync::{
            context::SyncContext,
            steps::{CleanupTombstonesStep, DeleteCloudFilesStep, UploadPendingFilesStep},
        },
        pipeline::pipeline_step::{PipelineStep, StepAction},
        settings_service::SettingsService,
        view_models::Settings,
    };

    #[async_std::test]
    async fn test_upload_pending_files_step() {
        let mut context = initialize_sync_context().await;

        let file_info_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                Some("file1.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();

        context.files_prepared_for_upload = 1;

        let step = UploadPendingFilesStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);

        let upload_result = context.upload_results.get("rom/file1.zst").unwrap();
        assert!(upload_result.cloud_operation_success);
        assert!(upload_result.db_update_success);

        // cloud_sync_status must now be Synced
        let file_info = context
            .repository_manager
            .get_file_info_repository()
            .get_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(file_info.cloud_sync_status, CloudSyncStatus::Synced);

        // UploadCompleted audit log entry must be written
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
    async fn test_upload_pending_files_step_more_than_one_batch() {
        let mut context = initialize_sync_context().await;

        let test_files = [
            ("file1.zst", [0; 20]),
            ("file2.zst", [1; 20]),
            ("file3.zst", [2; 20]),
            ("file4.zst", [3; 20]),
            ("file5.zst", [4; 20]),
            ("file6.zst", [5; 20]),
            ("file7.zst", [6; 20]),
            ("file8.zst", [7; 20]),
            ("file9.zst", [8; 20]),
            ("file10.zst", [9; 20]),
            ("file11.zst", [10; 20]),
        ];

        for (file_name, checksum) in test_files.iter() {
            add_file_info(
                &context.repository_manager,
                *checksum,
                file_name,
                FileType::Rom,
            )
            .await;
        }

        context.files_prepared_for_upload = 11;

        let not_synced_count = context
            .repository_manager
            .get_file_info_repository()
            .count_files_pending_upload()
            .await
            .unwrap();
        assert_eq!(not_synced_count, 11);

        let step = UploadPendingFilesStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);
        assert!(
            context
                .upload_results
                .get("rom/file1.zst")
                .unwrap()
                .cloud_operation_success
        );

        let not_synced_after = context
            .repository_manager
            .get_file_info_repository()
            .count_files_pending_upload()
            .await
            .unwrap();
        assert_eq!(not_synced_after, 0);
    }

    #[async_std::test]
    async fn test_delete_cloud_files_step() {
        let mut context = initialize_sync_context().await;
        let file_info_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                Some("file1.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();

        context
            .repository_manager
            .get_file_info_repository()
            .update_cloud_sync_status(file_info_id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        context.cloud_files_prepared_for_deletion = 1;
        let step = DeleteCloudFilesStep;
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);
        let deletion_result = context.deletion_results.get("rom/file1.zst").unwrap();
        assert!(deletion_result.cloud_operation_success);
        assert!(deletion_result.db_update_success);

        // file_info must be deleted after successful cloud deletion
        let res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_info(file_info_id)
            .await;
        assert!(res.is_err());

        // DeletionCompleted audit log entry must be written
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

    #[async_std::test]
    async fn test_delete_cloud_files_step_more_than_one_batch() {
        let mut context = initialize_sync_context().await;
        let test_files = [
            ("file1.zst", [0; 20]),
            ("file2.zst", [1; 20]),
            ("file3.zst", [2; 20]),
            ("file4.zst", [3; 20]),
            ("file5.zst", [4; 20]),
            ("file6.zst", [5; 20]),
            ("file7.zst", [6; 20]),
            ("file8.zst", [7; 20]),
            ("file9.zst", [8; 20]),
            ("file10.zst", [9; 20]),
            ("file11.zst", [10; 20]),
        ];
        for (file_name, checksum) in test_files.iter() {
            let id = add_file_info(
                &context.repository_manager,
                *checksum,
                file_name,
                FileType::Rom,
            )
            .await;
            set_sync_status(
                &context.repository_manager,
                id,
                CloudSyncStatus::DeletionPending,
            )
            .await;
        }

        let deletion_pending_count = context
            .repository_manager
            .get_file_info_repository()
            .count_files_pending_deletion()
            .await
            .unwrap();
        assert_eq!(deletion_pending_count, 11);

        context.cloud_files_prepared_for_deletion = 11;
        let step = DeleteCloudFilesStep;
        let action = step.execute(&mut context).await;
        assert_eq!(action, StepAction::Continue);
        assert!(
            context
                .deletion_results
                .get("rom/file1.zst")
                .unwrap()
                .cloud_operation_success
        );

        let deletion_pending_after = context
            .repository_manager
            .get_file_info_repository()
            .count_files_pending_deletion()
            .await
            .unwrap();
        assert_eq!(deletion_pending_after, 0);
    }

    #[async_std::test]
    async fn test_upload_pending_files_step_upload_failure() {
        let cloud_ops = Arc::new(MockCloudStorage::new());
        cloud_ops.fail_upload_for("rom/file1.zst");
        let mut context = initialize_sync_context_with_cloud(cloud_ops).await;

        let file_info_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                Some("file1.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();

        context.files_prepared_for_upload = 1;
        let step = UploadPendingFilesStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);

        let upload_result = context.upload_results.get("rom/file1.zst").unwrap();
        assert!(!upload_result.cloud_operation_success);

        // cloud_sync_status must remain NotSynced for automatic retry next sync
        let file_info = context
            .repository_manager
            .get_file_info_repository()
            .get_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(file_info.cloud_sync_status, CloudSyncStatus::NotSynced);

        // UploadFailed audit log must be written
        let log_entries = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(
            log_entries.first().unwrap().status,
            FileSyncStatus::UploadFailed
        );
    }

    #[async_std::test]
    async fn test_delete_cloud_files_step_deletion_failure() {
        let cloud_ops = Arc::new(MockCloudStorage::new());
        cloud_ops.fail_delete_for("rom/file1.zst");
        let mut context = initialize_sync_context_with_cloud(cloud_ops).await;

        let file_info_id = context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                Some("file1.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();

        context
            .repository_manager
            .get_file_info_repository()
            .update_cloud_sync_status(file_info_id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        context.cloud_files_prepared_for_deletion = 1;
        let step = DeleteCloudFilesStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);

        let deletion_result = context.deletion_results.get("rom/file1.zst").unwrap();
        assert!(!deletion_result.cloud_operation_success);

        // Tombstone file_info must be retained (DeletionPending) for retry next sync
        let file_info = context
            .repository_manager
            .get_file_info_repository()
            .get_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(
            file_info.cloud_sync_status,
            CloudSyncStatus::DeletionPending
        );

        // DeletionFailed audit log must be written
        let log_entries = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(
            log_entries.first().unwrap().status,
            FileSyncStatus::DeletionFailed
        );
    }

    async fn add_file_info(
        repo_manager: &RepositoryManager,
        checksum: [u8; 20],
        file_name: &str,
        file_type: FileType,
    ) -> i64 {
        repo_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from(checksum),
                1234,
                Some(file_name),
                file_type,
            )
            .await
            .unwrap()
    }

    async fn set_sync_status(
        repo_manager: &RepositoryManager,
        file_info_id: i64,
        status: CloudSyncStatus,
    ) {
        repo_manager
            .get_file_info_repository()
            .update_cloud_sync_status(file_info_id, status)
            .await
            .unwrap();
    }

    async fn initialize_sync_context() -> SyncContext {
        initialize_sync_context_with_cloud(Arc::new(MockCloudStorage::new())).await
    }

    async fn initialize_sync_context_with_cloud(cloud_ops: Arc<MockCloudStorage>) -> SyncContext {
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });

        let settings_service = Arc::new(SettingsService::new(repo_manager.clone()));

        let (tx, _rx) = flume::unbounded();
        let (_cancel_tx, cancel_rx) = flume::unbounded::<()>();

        SyncContext {
            settings,
            repository_manager: repo_manager,
            cloud_ops: Some(cloud_ops),
            progress_tx: tx,
            files_prepared_for_upload: 0,
            cloud_files_prepared_for_deletion: 0,
            tombstones_prepared_for_cleanup: 0,
            tombstones_cleaned_up: 0,
            upload_results: HashMap::new(),
            deletion_results: HashMap::new(),
            settings_service,
            cancel_rx,
        }
    }

    #[async_std::test]
    async fn test_upload_step_emits_upload_progress_events() {
        // Arrange
        let pool = Arc::new(setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(Settings {
            collection_root_dir: PathBuf::from("/"),
            ..Default::default()
        });
        let cloud_ops = Arc::new(MockCloudStorage::new());
        let (tx, rx) = flume::unbounded();
        let settings_service = Arc::new(SettingsService::new(repo_manager.clone()));
        let (_cancel_tx, cancel_rx) = flume::unbounded::<()>();

        repo_manager
            .get_file_info_repository()
            .add_file_info(
                &Sha1Checksum::from([0; 20]),
                1234,
                Some("file1.zst"),
                FileType::Rom,
            )
            .await
            .unwrap();

        let mut context = SyncContext {
            settings,
            repository_manager: repo_manager,
            cloud_ops: Some(cloud_ops),
            progress_tx: tx,
            files_prepared_for_upload: 1,
            cloud_files_prepared_for_deletion: 0,
            tombstones_prepared_for_cleanup: 0,
            tombstones_cleaned_up: 0,
            upload_results: HashMap::new(),
            deletion_results: HashMap::new(),
            settings_service,
            cancel_rx,
        };

        // Act
        let step = UploadPendingFilesStep;
        step.execute(&mut context).await;

        // Assert — lifecycle events (SyncStarted/SyncCompleted) are no longer emitted by this
        // step; only the per-file upload progress events should appear.
        let messages: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert_eq!(messages.len(), 5);

        assert!(matches!(messages[0], SyncEvent::FileUploadStarted {
            ref key, file_number: 1, total_files: 1
        } if key == "rom/file1.zst"));

        // mock simulates uploading in 3 parts by default
        assert!(matches!(messages[1], SyncEvent::PartUploaded {
            ref key, part: 1
        } if key == "rom/file1.zst"));
        assert!(matches!(messages[2], SyncEvent::PartUploaded {
            ref key, part: 2
        } if key == "rom/file1.zst"));
        assert!(matches!(messages[3], SyncEvent::PartUploaded {
            ref key, part: 3
        } if key == "rom/file1.zst"));

        assert!(matches!(messages[4], SyncEvent::FileUploadCompleted {
            ref key, file_number: 1, total_files: 1
        } if key == "rom/file1.zst"));
    }
}
