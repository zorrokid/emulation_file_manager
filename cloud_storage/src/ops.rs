use std::path::Path;

use async_std::channel::Sender;
use async_trait::async_trait;

use crate::{CloudStorageError, SyncEvent, events::DownloadEvent};

/// Trait for cloud storage operations to enable testing
#[async_trait]
pub trait CloudStorageOps: Send + Sync {
    /// Upload a file to cloud storage
    ///
    /// The implementation handles multipart upload logic internally.
    /// Progress events are sent through the optional progress_tx channel.
    async fn upload_file(
        &self,
        file_path: &Path,
        cloud_key: &str,
        progress_tx: Option<&Sender<SyncEvent>>,
    ) -> Result<(), CloudStorageError>;

    /// Delete a file from cloud storage
    async fn delete_file(&self, cloud_key: &str) -> Result<(), CloudStorageError>;

    /// Check if a file exists in cloud storage
    async fn file_exists(&self, cloud_key: &str) -> Result<bool, CloudStorageError>;

    /// Download a file from cloud storage to the specified destination path
    async fn download_file(
        &self,
        cloud_key: &str,
        destination_path: &Path,
        progress_tx: Option<&Sender<DownloadEvent>>,
    ) -> Result<(), CloudStorageError>;
}
