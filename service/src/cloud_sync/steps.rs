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

/// Step 1: Get counts of files to upload and delete from `file_info.cloud_sync_status`.
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
            .get_file_info_repository()
            .count_files_pending_deletion()
            .await;

        match files_to_delete_res {
            Ok(count) => {
                context.files_prepared_for_deletion = count;
                tracing::debug!(count, "Files prepared for deletion");
            }
            Err(e) => {
                tracing::error!(error = %e, "Error counting files for deletion");
                return StepAction::Abort(Error::DbError(e.to_string()));
            }
        }

        let files_pending_upload_res = context
            .repository_manager
            .get_file_info_repository()
            .count_files_pending_upload()
            .await;
        match files_pending_upload_res {
            Ok(count) => {
                context.files_prepared_for_upload = count;
                tracing::debug!(count, "Files prepared for upload");
            }
            Err(e) => {
                tracing::error!(error = %e, "Error counting files for upload");
                return StepAction::Abort(Error::DbError(e.to_string()));
            }
        }

        tracing::debug!("File counts retrieval completed");
        StepAction::Continue
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

        send_progress_event(
            SyncEvent::SyncStarted {
                total_files_count: context.files_prepared_for_upload,
            },
            &context.progress_tx,
        )
        .await;

        loop {
            let pending_files_result = context
                .repository_manager
                .get_file_info_repository()
                .get_files_pending_upload(
                    10,
                    0, // always offset 0: each uploaded file is updated to Synced so it
                       // won't appear in the next fetch
                )
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

                    // TODO: remove this by removing is_available and only relying on
                    // archive_file_name presence for upload eligibility.
                    let mut batch_uploaded = 0;
                    for file in pending_files {
                        if context.cancel_rx.try_recv().is_ok() {
                            tracing::info!("Cloud sync cancelled by user");
                            send_progress_event(SyncEvent::SyncCancelled {}, &context.progress_tx)
                                .await;
                            return StepAction::Abort(Error::OperationCancelled);
                        }

                        let Some(cloud_key) = file.generate_cloud_key() else {
                            // Invariant violation: is_available=true but archive_file_name=None.
                            // Status stays NotSynced so the file can be retried in a future sync
                            // once the data is corrected.
                            tracing::warn!(
                                file_info_id = file.id,
                                "File is available but has no archive_file_name; skipping upload"
                            );
                            continue;
                        };

                        tracing::debug!(
                            file_info_id = file.id,
                            cloud_key = %cloud_key,
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

                        let local_path = context.settings.get_file_path(
                            &file.file_type,
                            file.archive_file_name.as_deref().expect("checked above"),
                        );

                        tracing::debug!(
                            file_info_id = file.id,
                            local_path = %local_path.display(),
                            "Uploading file to cloud"
                        );

                        let upload_res = context
                            .cloud_ops
                            .as_ref()
                            .unwrap()
                            .upload_file(
                                local_path.as_path(),
                                &cloud_key,
                                Some(&context.progress_tx),
                            )
                            .await;

                        match upload_res {
                            Ok(_) => {
                                tracing::info!(
                                    file_info_id = file.id,
                                    cloud_key = %cloud_key,
                                    "Upload succeeded"
                                );
                                file_sync_result.cloud_operation_success = true;
                                batch_uploaded += 1;

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
                                    (Ok(_), Ok(_)) => {
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
                            }
                        }
                        context.upload_results.insert(cloud_key, file_sync_result);
                    }
                    // Break if no progress was made (e.g., all files lack archive_file_name)
                    // to avoid an infinite loop within a single sync session.
                    if batch_uploaded == 0 {
                        break;
                    }
                }
            }
        }
        send_progress_event(SyncEvent::SyncCompleted {}, &context.progress_tx).await;
        tracing::debug!("Pending file uploads completed");
        StepAction::Continue
    }
}

/// Step 3: Delete files with `cloud_sync_status = DeletionPending` from cloud storage.
/// On success, deletes the `file_info` record entirely (the tombstone is no longer needed).
/// On failure, leaves `cloud_sync_status = DeletionPending` for retry on next sync.
pub struct DeleteMarkedFilesStep;

#[async_trait::async_trait]
impl PipelineStep<SyncContext> for DeleteMarkedFilesStep {
    fn name(&self) -> &'static str {
        "delete_marked_files"
    }

    fn should_execute(&self, context: &SyncContext) -> bool {
        context.cloud_ops.is_some() && context.files_prepared_for_deletion > 0
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        tracing::debug!("Deleting marked files from cloud storage");
        let mut file_count = 0;

        loop {
            let pending_files_res = context
                .repository_manager
                .get_file_info_repository()
                .get_files_pending_deletion(
                    10,
                    0, // always offset 0: successfully deleted files are removed from file_info
                )
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

                    let mut batch_deleted = 0;
                    for file in pending_files {
                        if context.cancel_rx.try_recv().is_ok() {
                            tracing::info!("Cloud sync cancelled by user");
                            send_progress_event(SyncEvent::SyncCancelled, &context.progress_tx)
                                .await;
                            return StepAction::Abort(Error::OperationCancelled);
                        }

                        let Some(cloud_key) = file.generate_cloud_key() else {
                            tracing::warn!(
                                file_info_id = file.id,
                                "DeletionPending file has no archive_file_name; skipping"
                            );
                            continue;
                        };

                        send_progress_event(
                            SyncEvent::FileDeletionStarted {
                                key: cloud_key.clone(),
                                file_number: file_count,
                                total_files: context.files_prepared_for_deletion,
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

                        file_count += 1;
                        let deletion_res = context
                            .cloud_ops
                            .as_ref()
                            .unwrap()
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
                                        batch_deleted += 1;
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
                                        total_files: context.files_prepared_for_deletion,
                                    },
                                    &context.progress_tx,
                                )
                                .await;
                            }
                            Err(e) => {
                                tracing::error!(
                                    file_info_id = file.id,
                                    error = %e,
                                    "File deletion failed"
                                );
                                file_deletion_result.cloud_operation_success = false;
                                file_deletion_result.cloud_error = Some(format!("{}", e));

                                // Leave cloud_sync_status = DeletionPending for retry.
                                // Write audit log entry for diagnosis.
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
                                        total_files: context.files_prepared_for_deletion,
                                    },
                                    &context.progress_tx,
                                )
                                .await;
                            }
                        }
                        context
                            .deletion_results
                            .insert(cloud_key, file_deletion_result);
                    }
                    // Break if no progress was made to avoid infinite loop
                    // (e.g., all cloud deletions failed, files stay DeletionPending).
                    if batch_deleted == 0 {
                        break;
                    }
                }
            }
        }

        tracing::debug!("Marked file deletions completed");
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
            steps::{DeleteMarkedFilesStep, GetSyncFileCountsStep, UploadPendingFilesStep},
        },
        pipeline::pipeline_step::{PipelineStep, StepAction},
        settings_service::SettingsService,
        view_models::Settings,
    };

    #[async_std::test]
    async fn test_get_sync_file_counts_step() {
        let mut context = initialize_sync_context().await;

        let file_info_id_1 = add_file_info(
            &context.repository_manager,
            [0; 20],
            "file1.zst",
            FileType::Rom,
        )
        .await;
        let file_info_id_2 = add_file_info(
            &context.repository_manager,
            [1; 20],
            "file2.zst",
            FileType::Rom,
        )
        .await;
        let file_info_id_3 = add_file_info(
            &context.repository_manager,
            [2; 20],
            "file3.zst",
            FileType::Rom,
        )
        .await;
        let file_info_id_4 = add_file_info(
            &context.repository_manager,
            [3; 20],
            "file4.zst",
            FileType::Rom,
        )
        .await;
        let file_info_id_5 = add_file_info(
            &context.repository_manager,
            [4; 20],
            "file5.zst",
            FileType::Rom,
        )
        .await;

        // file1, file2: NotSynced (default) → counted for upload
        // file3, file4: DeletionPending → counted for deletion
        set_sync_status(
            &context.repository_manager,
            file_info_id_3,
            CloudSyncStatus::DeletionPending,
        )
        .await;
        set_sync_status(
            &context.repository_manager,
            file_info_id_4,
            CloudSyncStatus::DeletionPending,
        )
        .await;
        // file5: Synced → not counted for either
        set_sync_status(
            &context.repository_manager,
            file_info_id_5,
            CloudSyncStatus::Synced,
        )
        .await;

        let _ = (file_info_id_1, file_info_id_2); // referenced for clarity

        let step = GetSyncFileCountsStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, StepAction::Continue);
        assert_eq!(context.files_prepared_for_deletion, 2);
        assert_eq!(context.files_prepared_for_upload, 2);
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
    async fn test_upload_pending_files_step_handles_missing_archive_file_name() {
        // Invariant-violating file (is_available=true, archive_file_name=None) must not
        // cause an infinite loop; the step should break out after one batch with no progress.
        let mut context = initialize_sync_context().await;
        add_invariant_violating_file_info(&context, Sha1Checksum::from([0; 20])).await;

        context.files_prepared_for_upload = 1;

        let step = UploadPendingFilesStep;
        let action = step.execute(&mut context).await;

        // Step must not abort or loop forever
        assert_eq!(action, StepAction::Continue);
        // No upload was performed
        assert!(context.upload_results.is_empty());
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

        context.files_prepared_for_deletion = 1;
        let step = crate::cloud_sync::steps::DeleteMarkedFilesStep;
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
    async fn test_delete_marked_files_step_more_than_one_batch() {
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

        context.files_prepared_for_deletion = 11;
        let step = crate::cloud_sync::steps::DeleteMarkedFilesStep;
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
        assert_eq!(log_entries.first().unwrap().status, FileSyncStatus::UploadFailed);
    }

    #[async_std::test]
    async fn test_delete_marked_files_step_deletion_failure() {
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

        context.files_prepared_for_deletion = 1;
        let step = DeleteMarkedFilesStep;
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
        assert_eq!(file_info.cloud_sync_status, CloudSyncStatus::DeletionPending);

        // DeletionFailed audit log must be written
        let log_entries = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info_id)
            .await
            .unwrap();
        assert_eq!(log_entries.first().unwrap().status, FileSyncStatus::DeletionFailed);
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

    /// Creates a `file_info` row with `is_available=true` but `archive_file_name=NULL`,
    /// which is the invariant violation exercised by several guard branches.
    async fn add_invariant_violating_file_info(
        context: &SyncContext,
        checksum: Sha1Checksum,
    ) -> i64 {
        let repo = &context.repository_manager;
        let id = repo
            .get_file_info_repository()
            .add_file_info(&checksum, 1234, None, FileType::Rom)
            .await
            .unwrap();
        repo.get_file_info_repository()
            .update_is_available(id, None)
            .await
            .unwrap();
        id
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

        let (tx, rx) = flume::unbounded();
        let settings_service = Arc::new(SettingsService::new(repo_manager.clone()));
        let (_cancel_tx, cancel_rx) = flume::unbounded::<()>();

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

        context
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

        assert!(matches!(
            messages[0],
            SyncEvent::SyncStarted {
                total_files_count: 1
            }
        ));

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

        assert!(matches!(messages[5], SyncEvent::FileUploadCompleted {
            ref key, file_number: 1, total_files: 1
        } if key == "rom/file1.zst"));

        assert!(matches!(messages[6], SyncEvent::SyncCompleted {}));
    }
}
