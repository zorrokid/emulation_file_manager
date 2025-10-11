// User needs to set the following environment variables:
// - key id (AWS_ACCESS_KEY_ID)
// - application key (AWS_SECRET_ACCESS_KEY)
// - endpoint for cloud storage, for example: s3.eu-central-003.backblazeb2.com
// - region for cloud storage, for example: eu-central-003
// - bucket name, for example: my-efm-bucket
//

use std::path::Path;

use async_std::channel::Sender;
use async_std::io::{ReadExt, WriteExt};
use async_std::stream::StreamExt;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;
use s3::serde_types::Part;

#[derive(Debug, Clone)]
pub enum SyncEvent {
    Started { key: String },
    PartUploaded { key: String, part: u32 },
    Completed { key: String },
    Failed { key: String, error: String },
}

#[derive(Debug, thiserror::Error)]
pub enum CloudStorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("S3 error: {0}")]
    S3(#[from] S3Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub async fn connect_bucket() -> Result<Box<Bucket>, CloudStorageError> {
    let region = Region::Custom {
        region: "eu-central-003".into(),
        endpoint: "s3.eu-central-003.backblazeb2.com".into(),
    };

    let credentials = Credentials::default()
        .map_err(|e| CloudStorageError::Other(format!("Credentials error: {e}")))?;

    let bucket = Bucket::new("efm-files", region, credentials)?.with_path_style();

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

async fn download_file(
    bucket: &Bucket,
    local_path: &Path,
    key: &str,
) -> Result<(), CloudStorageError> {
    let response_stream = bucket.get_object_stream(key).await?;
    let mut file = async_std::fs::File::create(local_path).await?;

    let mut bytes_stream = response_stream.bytes();

    while let Some(chunk) = bytes_stream.next().await {
        let data = chunk?;
        file.write_all(&[data]).await?;
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

    if let Some(tx) = &progress_tx {
        tx.send(SyncEvent::Started {
            key: key.to_string(),
        })
        .await
        .ok();
    }

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
                    tx.send(SyncEvent::Failed {
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
