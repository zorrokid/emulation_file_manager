use std::path::Path;

use cloud_storage::{connect_bucket, multipart_upload, CloudStorageError};

pub struct FileUpload {
    pub local_path: String,
    pub cloud_key: String,
}

pub async fn sync_files_to_cloud(files_to_upload: &[FileUpload]) -> Result<(), CloudStorageError> {
    let bucket = connect_bucket().await?;

    for file in files_to_upload {
        let local_path = Path::new(&file.local_path);
        multipart_upload(&bucket, local_path, &file.cloud_key).await?;
    }

    Ok(())
}
