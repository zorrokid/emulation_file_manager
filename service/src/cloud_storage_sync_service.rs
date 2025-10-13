use std::{path::PathBuf, sync::Arc};

use async_std::channel::Sender;
use cloud_storage::{connect_bucket, multipart_upload, CloudStorageError, SyncEvent};
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

    /// Goes though the list of files. If file info is missing in sync log, add an entry for it with
    /// pending status and creates a cloud key for file.
    /// Processes file infos in batches of 1000.
    pub async fn prepare_files_for_sync(&self) -> Result<(), CloudStorageError> {
        println!("Preparing files for sync...");
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
                        FileSyncStatus::Pending,
                        "",
                        cloud_key.as_str(),
                    )
                    .await
                    .map_err(|e| CloudStorageError::Other(e.to_string()))?;
            }
        }

        println!("Preparing files for sync...DONE");
        Ok(())
    }

    /// Goes through the list of pending and failed files and uploads them to cloud storage
    pub async fn sync_files_to_cloud(
        &self,
        progress_tx: Sender<SyncEvent>,
    ) -> Result<(), CloudStorageError> {
        self.prepare_files_for_sync().await?;
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

        let mut offset = 0;

        loop {
            let pending_files_to_upload = self
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_and_file_info_by_sync_status(10, offset)
                .await
                .map_err(|e| CloudStorageError::Other(e.to_string()))?;

            if pending_files_to_upload.is_empty() {
                break;
            }

            offset += pending_files_to_upload.len() as u32;

            for file in pending_files_to_upload {
                self.repository_manager
                    .get_file_sync_log_repository()
                    .update_log_entry(file.id, FileSyncStatus::InProgress, "")
                    .await
                    .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                let folder_name = file.file_type.dir_name();
                let local_path = PathBuf::from(&self.settings.collection_root_dir)
                    .join(folder_name)
                    .join(format!("{}.{}", &file.archive_file_name, "zst"));
                println!("Local path: {:?}", local_path);
                println!(
                    "Uploading file to cloud: id={}, key={}",
                    file.id, file.cloud_key
                );
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
                            .update_log_entry(file.id, FileSyncStatus::Completed, "")
                            .await
                            .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                    }
                    Err(e) => {
                        println!(
                            "Upload failed: id={}, key={}, error={}",
                            file.id, file.cloud_key, e
                        );
                        self.repository_manager
                            .get_file_sync_log_repository()
                            .update_log_entry(file.id, FileSyncStatus::Failed, &format!("{}", e))
                            .await
                            .map_err(|e| CloudStorageError::Other(e.to_string()))?;
                    }
                }
            }
        }

        Ok(())
    }
}
