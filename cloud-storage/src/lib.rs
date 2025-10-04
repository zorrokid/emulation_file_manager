// User needs to set the following environment variables:
// - key id (AWS_ACCESS_KEY_ID)
// - application key (AWS_SECRET_ACCESS_KEY)
// - endpoint for cloud storage, for example: s3.eu-central-003.backblazeb2.com
// - region for cloud storage, for example: eu-central-003
// - bucket name, for example: my-efm-bucket
//

use std::path::Path;

use async_std::io::WriteExt;
use async_std::stream::StreamExt;
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::error::S3Error;
use s3::region::Region;

#[derive(Debug, thiserror::Error)]
pub enum CloudStorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("S3 error: {0}")]
    S3(#[from] S3Error),

    #[error("Other error: {0}")]
    Other(String),
}

async fn connect_bucket() -> Result<Box<Bucket>, CloudStorageError> {
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
    let mut response_stream = bucket.get_object_stream(key).await?;
    let mut file = async_std::fs::File::create(local_path).await?;

    while let Some(chunk) = response_stream.bytes().next().await {
        let data = chunk?;
        file.write_all(&data).await?;
    }

    Ok(())
}
