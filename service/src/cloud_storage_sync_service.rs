use std::sync::Arc;

use async_std::channel::Sender;
use cloud_storage::{connect_bucket, delete_file, multipart_upload, CloudStorageError, SyncEvent};
use core_types::FileSyncStatus;
use database::repository_manager::RepositoryManager;

use crate::view_models::Settings;

pub struct FileUpload {
    pub local_path: String,
    pub cloud_key: String,
}

#[derive(Debug)]
pub struct CloudStorageSyncService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
}

impl CloudStorageSyncService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self {
            repository_manager,
            settings,
        }
    }

    pub async fn sync(&self, progress_tx: Sender<SyncEvent>) -> Result<(), CloudStorageError> {
        let count = self.prepare_files_for_sync().await?;
        self.sync_files_to_cloud(progress_tx.clone(), count).await?;
        self.delete_files_from_cloud(progress_tx.clone()).await?;
        Ok(())
    }

    // STEP 1
    /// Goes though the list of files. If file info is missing in sync log, add an entry for it with
    /// pending status and creates a cloud key for file.
    /// Processes file infos in batches of 1000.
    async fn prepare_files_for_sync(&self) -> Result<i64, CloudStorageError> {
        println!("Preparing files for sync...");
        let mut total_count: i64 = 0;
        let mut offset = 0;
        loop {
            let file_infos = self
                .repository_manager
                .get_file_info_repository()
                .get_file_infos_without_sync_log(100, offset)
                .await
                .map_err(|e| CloudStorageError::Other(e.to_string()))?;

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
                self.repository_manager
                    .get_file_sync_log_repository()
                    .add_log_entry(
                        file_info.id,
                        FileSyncStatus::UploadPending,
                        "",
                        cloud_key.as_str(),
                    )
                    .await
                    .map_err(|e| CloudStorageError::Other(e.to_string()))?;
            }
        }

        println!("Preparing files for sync...DONE");
        Ok(total_count)
    }

    // STEP 2
    /// Goes through the list of pending and failed files and uploads them to cloud storage
    async fn sync_files_to_cloud(
        &self,
        progress_tx: Sender<SyncEvent>,
        total_files_count: i64,
    ) -> Result<(), CloudStorageError> {
        let mut successful_files_count = 0;
        let mut failed_files_count = 0;
        let mut file_count = 0;
        progress_tx
            .send(SyncEvent::SyncStarted { total_files_count })
            .await
            .ok();

        let bucket = self.get_bucket().await?;
        let mut offset = 0;

        loop {
            let pending_files_to_upload = self
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_and_file_info_by_sync_status(
                    &[FileSyncStatus::UploadPending, FileSyncStatus::UploadFailed],
                    10,
                    offset,
                )
                .await
                .map_err(|e| CloudStorageError::Other(e.to_string()))?;

            if pending_files_to_upload.is_empty() {
                break;
            }

            offset += pending_files_to_upload.len() as u32;

            for file in pending_files_to_upload {
                self.repository_manager
                    .get_file_sync_log_repository()
                    .add_log_entry(
                        file.id,
                        FileSyncStatus::UploadInProgress,
                        "",
                        &file.cloud_key,
                    )
                    .await
                    .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                let local_path = self
                    .settings
                    .get_file_path(&file.file_type, &file.archive_file_name);
                println!("Local path: {:?}", local_path);
                println!(
                    "Uploading file to cloud: id={}, key={}",
                    file.id, file.cloud_key
                );
                file_count += 1;
                progress_tx
                    .send(SyncEvent::FileUploadStarted {
                        key: file.cloud_key.clone(),
                        file_number: file_count,
                        total_files: total_files_count,
                    })
                    .await
                    .ok();
                let res = multipart_upload(
                    &bucket,
                    local_path.as_path(),
                    &file.cloud_key,
                    Some(&progress_tx),
                )
                .await;
                match res {
                    Ok(_) => {
                        println!("Upload completed: id={}, key={}", file.id, file.cloud_key);
                        self.repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file.id,
                                FileSyncStatus::UploadCompleted,
                                "",
                                &file.cloud_key,
                            )
                            .await
                            .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                        successful_files_count += 1;
                        progress_tx
                            .send(SyncEvent::FileUploadCompleted {
                                key: file.cloud_key.clone(),
                                file_number: file_count,
                                total_files: total_files_count,
                            })
                            .await
                            .ok();
                    }
                    Err(e) => {
                        println!(
                            "Upload failed: id={}, key={}, error={}",
                            file.id, file.cloud_key, e
                        );
                        self.repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file.id,
                                FileSyncStatus::UploadFailed,
                                &format!("{}", e),
                                &file.cloud_key,
                            )
                            .await
                            .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                        failed_files_count += 1;
                        progress_tx
                            .send(SyncEvent::FileUploadFailed {
                                key: file.cloud_key.clone(),
                                error: format!("{}", e),
                                file_number: file_count,
                                total_files: total_files_count,
                            })
                            .await
                            .ok();
                    }
                }
            }
        }
        progress_tx.send(SyncEvent::SyncCompleted {}).await.ok();
        Ok(())
    }

    // STEP 3
    /// Goes through the list of files marked for deletion and deletes them from cloud storage
    pub async fn delete_files_from_cloud(
        &self,
        progress_tx: Sender<SyncEvent>,
    ) -> Result<(), CloudStorageError> {
        let total_files_count = self
            .repository_manager
            .get_file_sync_log_repository()
            .count_logs_by_latest_status(FileSyncStatus::DeletionPending)
            .await
            .map_err(|e| CloudStorageError::Other(e.to_string()))?;

        let mut successful_files_count = 0;
        let mut failed_files_count = 0;
        let mut file_count = 0;

        let bucket = self.get_bucket().await?;
        let mut offset = 0;

        loop {
            let pending_files = self
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
                .await
                .map_err(|e| CloudStorageError::Other(e.to_string()))?;

            if pending_files.is_empty() {
                break;
            }

            offset += pending_files.len() as u32;

            for file in pending_files {
                self.repository_manager
                    .get_file_sync_log_repository()
                    .add_log_entry(
                        file.id,
                        FileSyncStatus::DeletionInProgress,
                        "",
                        &file.cloud_key,
                    )
                    .await
                    .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                file_count += 1;
                progress_tx
                    .send(SyncEvent::FileDeletionStarted {
                        key: file.cloud_key.clone(),
                        file_number: file_count,
                        total_files: total_files_count,
                    })
                    .await
                    .ok();

                let res = delete_file(&bucket, &file.cloud_key).await;

                match res {
                    Ok(_) => {
                        println!("Deletion completed: id={}, key={}", file.id, file.cloud_key);
                        self.repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file.id,
                                FileSyncStatus::DeletionCompleted,
                                "",
                                &file.cloud_key,
                            )
                            .await
                            .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                        successful_files_count += 1;
                        progress_tx
                            .send(SyncEvent::FileDeletionCompleted {
                                key: file.cloud_key.clone(),
                                file_number: file_count,
                                total_files: total_files_count,
                            })
                            .await
                            .ok();
                    }
                    Err(e) => {
                        println!(
                            "Deletion failed: id={}, key={}, error={}",
                            file.id, file.cloud_key, e
                        );
                        self.repository_manager
                            .get_file_sync_log_repository()
                            .add_log_entry(
                                file.id,
                                FileSyncStatus::DeletionFailed,
                                &format!("{}", e),
                                &file.cloud_key,
                            )
                            .await
                            .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                        failed_files_count += 1;
                        progress_tx
                            .send(SyncEvent::FileDeletionFailed {
                                key: file.cloud_key.clone(),
                                error: format!("{}", e),
                                file_number: file_count,
                                total_files: total_files_count,
                            })
                            .await
                            .ok();
                    }
                }
            }
        }

        Ok(())
    }

    async fn get_bucket(&self) -> Result<Box<cloud_storage::Bucket>, CloudStorageError> {
        let s3_settings = match &self.settings.s3_settings {
            Some(s) => s,
            None => {
                return Err(CloudStorageError::Other(
                    "S3 settings are not configured".to_string(),
                ))
            }
        };
        let bucket = connect_bucket(
            &s3_settings.endpoint,
            &s3_settings.region,
            &s3_settings.bucket,
        )
        .await?;

        Ok(bucket)
    }
}
