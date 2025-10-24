use std::sync::Arc;

use cloud_storage::{S3CloudStorage, SyncEvent};
use core_types::FileSyncStatus;

use crate::{
    cloud_sync::{
        context::SyncContext,
        pipeline::{CloudStorageSyncStep, StepAction},
    },
    error::Error,
};

/// Step 1: Prepare files for upload. This involves marking files as pending upload in the
/// database and collecting total number of files to be uploaded.
pub struct PrepareFilesForUploadStep;

#[async_trait::async_trait]
impl CloudStorageSyncStep for PrepareFilesForUploadStep {
    fn name(&self) -> &'static str {
        "prepare_files"
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        println!("PrepareFilesStep");
        let mut total_count: i64 = 0;
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
                    total_count += file_infos.len() as i64;
                    for file_info in &file_infos {
                        let cloud_key = format!(
                            "{}/{}",
                            file_info.file_type.to_string().to_lowercase(),
                            file_info.archive_file_name
                        );
                        println!(
                            "Preparing file for sync: id={}, cloud_key={}",
                            file_info.id, cloud_key
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
                            eprintln!(
                                "Error updating sync log for file_info id {}: {}",
                                file_info.id, e
                            );

                            return StepAction::Abort(Error::DbError(e.to_string()));
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching file infos: {}", e);
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
            }
        }

        println!("Preparing files for sync...DONE");

        context.files_prepared_for_upload = total_count;
        StepAction::Continue
    }
}

pub struct PrepareFilesForDeletionStep;

#[async_trait::async_trait]
impl CloudStorageSyncStep for PrepareFilesForDeletionStep {
    fn name(&self) -> &'static str {
        "prepare_files_for_deletion"
    }
    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        println!("PrepareFilesForDeletionStep");

        let files_to_delete_res = context
            .repository_manager
            .get_file_sync_log_repository()
            .count_logs_by_latest_status(FileSyncStatus::DeletionPending)
            .await;

        match files_to_delete_res {
            Ok(count) => {
                context.files_prepared_for_deletion = count;
                println!("Files prepared for deletion: {}", count);
                StepAction::Continue
            }
            Err(e) => {
                eprintln!("Error counting files for deletion: {}", e);
                StepAction::Abort(Error::DbError(e.to_string()))
            }
        }
    }
}

/// Step 2: Connect to cloud cloud_storage
pub struct ConnectToCloudStep;

#[async_trait::async_trait]
impl CloudStorageSyncStep for ConnectToCloudStep {
    fn name(&self) -> &'static str {
        "connect_to_cloud"
    }

    fn should_execute(&self, context: &SyncContext) -> bool {
        context.cloud_ops.is_none()
            && (context.files_prepared_for_upload > 0 || context.files_prepared_for_deletion > 0)
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        let s3_settings = match context.settings.s3_settings.clone() {
            Some(settings) => settings,
            None => {
                eprintln!("S3 settings are not configured.");
                return StepAction::Abort(Error::SettingsError("S3 settings missing".to_string()));
            }
        };
        let cloud_ops_res = S3CloudStorage::connect(
            s3_settings.endpoint.as_str(),
            s3_settings.region.as_str(),
            s3_settings.bucket.as_str(),
        )
        .await;

        match cloud_ops_res {
            Ok(cloud_ops) => {
                context.cloud_ops = Some(Arc::new(cloud_ops));
                StepAction::Continue
            }
            Err(e) => {
                eprintln!("Error connecting to S3: {}", e);
                StepAction::Abort(Error::CloudSyncError(format!(
                    "Failed to connect to S3: {}",
                    e
                )))
            }
        }
    }
}

/// Step 3: Upload pending files to cloud storage
pub struct UploadPendingFilesStep;

#[async_trait::async_trait]
impl CloudStorageSyncStep for UploadPendingFilesStep {
    fn name(&self) -> &'static str {
        "upload_pending_files"
    }

    fn should_execute(&self, context: &SyncContext) -> bool {
        context.cloud_ops.is_some() && context.files_prepared_for_upload > 0
    }

    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        let mut successful_files_count = 0;
        let mut failed_files_count = 0;
        let mut file_count = 0;

        context
            .progress_tx
            .send(SyncEvent::SyncStarted {
                total_files_count: context.files_prepared_for_upload,
            })
            .await
            .ok();

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
                    eprintln!("Error fetching pending files to upload: {}", e);
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(pending_files) => {
                    if pending_files.is_empty() {
                        break;
                    }

                    offset += pending_files.len() as u32;

                    for file in pending_files {
                        let update_res = context
                            .repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file.id,
                                FileSyncStatus::UploadInProgress,
                                "",
                                &file.cloud_key,
                            )
                            .await;

                        if let Err(e) = update_res {
                            eprintln!(
                                "Error updating sync log to UploadInProgress for file_info id {}: {}",
                                file.id, e
                            );
                            return StepAction::Abort(Error::DbError(e.to_string()));
                        }

                        let local_path = context
                            .settings
                            .get_file_path(&file.file_type, &file.archive_file_name);

                        println!("Local path: {:?}", local_path);
                        println!(
                            "Uploading file to cloud: id={}, key={}",
                            file.id, file.cloud_key
                        );
                        file_count += 1;

                        context
                            .progress_tx
                            .send(SyncEvent::FileUploadStarted {
                                key: file.cloud_key.clone(),
                                file_number: file_count,
                                total_files: context.files_prepared_for_upload,
                            })
                            .await
                            .ok();

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
                                println!(
                                    "Upload completed: id={}, key={}",
                                    file.id, file.cloud_key
                                );

                                context.upload_results.insert(
                                    file.cloud_key.clone(),
                                    crate::cloud_sync::context::FileSyncResult {
                                        file_info_id: file.id,
                                        cloud_key: file.cloud_key.clone(),
                                        success: true,
                                        error_message: None,
                                    },
                                );

                                let update_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::UploadCompleted,
                                        "",
                                        &file.cloud_key,
                                    )
                                    .await;

                                // TODO: handle result properly
                                if let Err(e) = update_res {
                                    eprintln!(
                                        "Error updating sync log to UploadCompleted for file_info id {}: {}",
                                        file.id, e
                                    );
                                }

                                successful_files_count += 1;

                                let update_res = context
                                    .progress_tx
                                    .send(SyncEvent::FileUploadCompleted {
                                        key: file.cloud_key.clone(),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_upload,
                                    })
                                    .await;

                                // TODO: handle result properly
                                if let Err(e) = update_res {
                                    eprintln!(
                                        "Error sending FileUploadCompleted event for key {}: {}",
                                        file.cloud_key, e
                                    );
                                }
                            }
                            Err(e) => {
                                println!(
                                    "Upload failed: id={}, key={}, error={}",
                                    file.id, file.cloud_key, e
                                );

                                context.upload_results.insert(
                                    file.cloud_key.clone(),
                                    crate::cloud_sync::context::FileSyncResult {
                                        file_info_id: file.id,
                                        cloud_key: file.cloud_key.clone(),
                                        success: false,
                                        error_message: Some(format!("{}", e)),
                                    },
                                );

                                let update_res = context
                                    .repository_manager
                                    .get_file_sync_log_repository()
                                    .add_log_entry(
                                        file.id,
                                        FileSyncStatus::UploadFailed,
                                        &format!("{}", e),
                                        &file.cloud_key,
                                    )
                                    .await;

                                // TODO: handle result properly
                                if let Err(e) = update_res {
                                    eprintln!(
                                        "Error updating sync log to UploadFailed for file_info id {}: {}",
                                        file.id, e
                                    );
                                }

                                failed_files_count += 1;

                                context
                                    .progress_tx
                                    .send(SyncEvent::FileUploadFailed {
                                        key: file.cloud_key.clone(),
                                        error: format!("{}", e),
                                        file_number: file_count,
                                        total_files: context.files_prepared_for_upload,
                                    })
                                    .await
                                    .ok();
                            }
                        }
                    }
                }
            }
        }
        context
            .progress_tx
            .send(SyncEvent::SyncCompleted {
                successful: successful_files_count,
                failed: failed_files_count,
            })
            .await
            .ok();
        StepAction::Continue
    }
}

/// Step 4: Delete the files marked for deletion from cloud storage
pub struct DeleteMarkedFilesStep;

#[async_trait::async_trait]
impl CloudStorageSyncStep for DeleteMarkedFilesStep {
    fn name(&self) -> &'static str {
        "delete_marked_files"
    }
    async fn execute(&self, context: &mut SyncContext) -> StepAction {
        // Implementation for deleting files goes here
        StepAction::Continue
    }
}
