use std::sync::Arc;

use cloud_storage::{S3CloudStorage, SyncEvent};
use core_types::FileSyncStatus;

use crate::{
    cloud_sync::{
        context::{FileSyncResult, SyncContext},
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
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(pending_files) => {
                    if pending_files.is_empty() {
                        break;
                    }

                    offset += pending_files.len() as u32;

                    for file in pending_files {
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
                                FileSyncStatus::UploadInProgress,
                                "",
                                &file.cloud_key,
                            )
                            .await;

                        if let Err(e) = update_res {
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
                                .ok();

                            // Skip this file and continue with the next one, since status update
                            // failed this will be retried in the next sync run
                            continue;
                        }

                        let local_path = context
                            .settings
                            .get_file_path(&file.file_type, &file.archive_file_name);

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
                                file_sync_result.cloud_operation_success = true;

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

                                match update_res {
                                    Ok(_) => {
                                        file_sync_result.db_update_success = true;
                                    }
                                    Err(e) => {
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
                                    .ok();
                            }
                            Err(e) => {
                                file_sync_result.cloud_operation_success = false;
                                file_sync_result.cloud_error = Some(format!("{}", e));

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

                                match update_res {
                                    Ok(_) => {
                                        file_sync_result.db_update_success = true;
                                    }
                                    Err(e) => {
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
                                    .ok();
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
                    return StepAction::Abort(Error::DbError(e.to_string()));
                }
                Ok(pending_files) => {
                    if pending_files.is_empty() {
                        break;
                    }

                    offset += pending_files.len() as u32;

                    for file in pending_files {
                        context
                            .progress_tx
                            .send(SyncEvent::FileDeletionStarted {
                                key: file.cloud_key.clone(),
                                file_number: file_count,
                                total_files: context.files_prepared_for_deletion,
                            })
                            .await
                            .ok();

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
                                .ok();

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
                                    .ok();
                            }
                            Err(e) => {
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
                                    .ok();
                            }
                        }
                        context
                            .deletion_results
                            .insert(file.cloud_key.clone(), file_deletion_result);
                    }
                }
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use cloud_storage::mock::MockCloudStorage;
    use core_types::{FileSyncStatus, FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        cloud_sync::{
            context::SyncContext,
            pipeline::{CloudStorageSyncStep, StepAction},
            steps::{PrepareFilesForDeletionStep, PrepareFilesForUploadStep},
        },
        file_system_ops::mock::MockFileSystemOps,
        view_models::Settings,
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

        assert_eq!(action, crate::cloud_sync::pipeline::StepAction::Continue);
        assert_eq!(context.files_prepared_for_upload, 1);

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

        assert_eq!(pending_files_to_upload_result.len(), 1);
        assert_eq!(
            pending_files_to_upload_result[0].archive_file_name,
            "file1.zst"
        );

        assert_eq!(file_infos_res.len(), 0);
    }

    #[async_std::test]
    async fn test_prepare_files_for_deletion_step() {
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

        let step = PrepareFilesForDeletionStep;
        let action = step.execute(&mut context).await;

        assert_eq!(action, crate::cloud_sync::pipeline::StepAction::Continue);
        assert_eq!(context.files_prepared_for_deletion, 1);
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
        let cloud_ops = Arc::new(MockCloudStorage::new());

        let (tx, _rx) = async_std::channel::unbounded();

        SyncContext {
            settings,
            repository_manager: repo_manager,
            cloud_ops: Some(cloud_ops),
            progress_tx: tx,
            files_prepared_for_upload: 0,
            files_prepared_for_deletion: 0,
            upload_results: HashMap::new(),
            deletion_results: HashMap::new(),
        }
    }
}
