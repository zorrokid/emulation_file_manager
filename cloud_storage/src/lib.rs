use std::path::Path;

use async_std::channel::Sender;
use async_std::io::WriteExt;
use async_std::stream::StreamExt;
use async_trait::async_trait;
pub use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;
use s3::serde_types::Part;

// Re-export Bucket so it can be used by consumers
pub use s3::bucket::Bucket as S3Bucket;

mod ops;
pub use ops::CloudStorageOps;

pub mod events;
pub use events::{DownloadEvent, SyncEvent};
pub mod mock;

#[derive(Debug, thiserror::Error)]
pub enum CloudStorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("S3 error: {0}")]
    S3(#[from] S3Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub async fn connect_bucket(
    endpoint: &str,
    region: &str,
    bucket: &str,
    key_id: &str,
    secret_key: &str,
) -> Result<Box<Bucket>, CloudStorageError> {
    let region = Region::Custom {
        region: region.to_string(),
        endpoint: endpoint.to_string(),
    };

    let credentials = Credentials::new(Some(key_id), Some(secret_key), None, None, None)
        .map_err(|_| CloudStorageError::Other("Credentials error".to_string()))?;

    let bucket = Bucket::new(bucket, region, credentials)?.with_path_style();

    Ok(bucket)
}

async fn upload_file(
    bucket: &Bucket,
    file_path: &Path,
    key: &str,
) -> Result<(), CloudStorageError> {
    let mut file = async_std::fs::File::open(file_path).await?;
    bucket.put_object_stream(&mut file, key).await?;
    Ok(())
}

/// Download a file from the bucket to the specified local path.
/// If a progress_tx channel is provided, send progress events during the download.
/// Doesn't send progress events from failed download or write operation but instead returns an
/// error immediately. The caller can handle the error and send any necessary events.
/// Also it's caller's responsibility to send start and completion events.
///
/// # Arguments
/// - `bucket`: Reference to the S3 bucket
/// - `local_path`: Path to save the downloaded file
/// - `key`: Key of the file in the bucket
/// - `progress_tx`: Optional channel sender for progress events
/// Returns `Ok(())` on success or `CloudStorageError` on failure.
async fn download_file(
    bucket: &Bucket,
    local_path: &Path,
    key: &str,
    progress_tx: Option<&Sender<DownloadEvent>>,
) -> Result<(), CloudStorageError> {
    let mut response_stream = bucket.get_object_stream(key).await?;
    let mut file = async_std::fs::File::create(local_path).await?;

    while let Some(chunk_res) = response_stream.bytes.next().await {
        let chunk = chunk_res?;
        file.write_all(&chunk).await?;
        if let Some(tx) = progress_tx {
            tx.send(DownloadEvent::FileDownloadProgress {
                key: key.to_string(),
                bytes_downloaded: chunk.len() as u64,
            })
            .await
            .ok();
        }
    }

    Ok(())
}

pub async fn multipart_upload(
    bucket: &Bucket,
    file_path: &Path,
    key: &str,
    progress_tx: Option<&Sender<SyncEvent>>,
) -> Result<(), CloudStorageError> {
    use async_std::io::ReadExt;

    let mut file = async_std::fs::File::open(file_path).await?;
    // 5 MB chunk size
    let mut buffer = vec![0u8; 5 * 1024 * 1024];
    let mut part_number = 1;
    let mut parts: Vec<Part> = Vec::new();

    let content_type = "application/zstd";

    let response = bucket.initiate_multipart_upload(key, content_type).await?;

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }

        println!("Uploading part {} ({} bytes)", part_number, bytes_read);
        let result = bucket
            .put_multipart_chunk(
                buffer[..bytes_read].to_vec(),
                key,
                part_number,
                &response.upload_id,
                content_type,
            )
            .await;
        println!("Finished part {} upload", part_number);

        match result {
            Ok(part) => {
                println!("Uploaded part {}: {:?}", part_number, part);
                if let Some(tx) = &progress_tx {
                    tx.send(SyncEvent::PartUploaded {
                        key: key.to_string(),
                        part: part_number,
                    })
                    .await
                    .ok();
                }
                parts.push(part);
                part_number += 1;
            }
            Err(e) => {
                eprintln!("Error uploading part {}: {}", part_number, e);
                if let Some(tx) = &progress_tx {
                    tx.send(SyncEvent::PartUploadFailed {
                        key: key.to_string(),
                        error: format!("{}", e),
                    })
                    .await
                    .ok();
                }
                bucket.abort_upload(key, &response.upload_id).await.ok();
                return Err(CloudStorageError::S3(e));
            }
        };
    }
    Ok(())
}

pub async fn delete_file(bucket: &Bucket, key: &str) -> Result<(), CloudStorageError> {
    bucket.delete_object(key).await?;
    Ok(())
}

pub struct S3CloudStorage {
    bucket: Box<Bucket>,
}

impl S3CloudStorage {
    /// Connect to an S3-compatible storage bucket
    pub async fn connect(
        endpoint: &str,
        region: &str,
        bucket_name: &str,
        key_id: &str,
        secret_key: &str,
    ) -> Result<Self, CloudStorageError> {
        let bucket = connect_bucket(endpoint, region, bucket_name, key_id, secret_key).await?;
        Ok(Self { bucket })
    }

    /// Get a reference to the underlying bucket
    pub fn bucket(&self) -> &Bucket {
        &self.bucket
    }
}

#[async_trait]
impl CloudStorageOps for S3CloudStorage {
    async fn upload_file(
        &self,
        file_path: &Path,
        cloud_key: &str,
        progress_tx: Option<&Sender<SyncEvent>>,
    ) -> Result<(), CloudStorageError> {
        multipart_upload(&self.bucket, file_path, cloud_key, progress_tx).await
    }

    async fn delete_file(&self, cloud_key: &str) -> Result<(), CloudStorageError> {
        delete_file(&self.bucket, cloud_key).await
    }

    async fn file_exists(&self, cloud_key: &str) -> Result<bool, CloudStorageError> {
        match self.bucket.head_object(cloud_key).await {
            Ok(_) => Ok(true),
            Err(S3Error::HttpFailWithBody(404, _)) => Ok(false),
            Err(e) => Err(CloudStorageError::S3(e)),
        }
    }

    async fn download_file(
        &self,
        cloud_key: &str,
        destination_path: &Path,
        progress_tx: Option<&Sender<DownloadEvent>>,
    ) -> Result<(), CloudStorageError> {
        download_file(&self.bucket, destination_path, cloud_key, progress_tx).await
    }
}
