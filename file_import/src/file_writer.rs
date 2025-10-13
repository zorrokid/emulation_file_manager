use s3::{bucket::Bucket, serde_types::Part};
use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::Path,
};

use core_types::{FileSize, Sha1Checksum};
use sha1::{
    digest::{consts::U20, generic_array::GenericArray},
    Digest, Sha1,
};
use zstd::Encoder;

use crate::compression_utils::CompressionLevel;

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

pub trait FileWriter {
    fn output_zstd_compressed<R: Read>(
        output_dir: &Path,
        file: &mut R,
        archive_file_name: &str,
        compression_level: CompressionLevel,
    ) -> Result<(Sha1Checksum, FileSize), Box<dyn std::error::Error>>;
}

pub struct LocalFileWriter;
pub struct CloudFileWriter;

impl FileWriter for LocalFileWriter {
    fn output_zstd_compressed<R: Read>(
        output_dir: &Path,
        file: &mut R,
        archive_file_name: &str,
        compression_level: CompressionLevel,
    ) -> Result<(Sha1Checksum, FileSize), Box<dyn std::error::Error>> {
        let zstd_file_path = output_dir.join(archive_file_name).with_extension("zst");
        if let Some(parent) = zstd_file_path.parent() {
            create_dir_all(parent)?;
        }
        let zstd_file = File::create(zstd_file_path)?;
        let mut encoder = Encoder::new(zstd_file, compression_level.to_zstd_level())?;
        let mut buffer = [0u8; 8192]; // 8 KB buffer
        let mut hasher = Sha1::new();
        let mut size: u64 = 0;

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // EOF
            }
            size += bytes_read as u64;
            hasher.update(&buffer[..bytes_read]);
            encoder.write_all(&buffer[..bytes_read])?;
        }
        encoder.finish()?;
        let checksum: GenericArray<u8, U20> = hasher.finalize();
        let checksum: Sha1Checksum = checksum.into();
        Ok((checksum, size))
    }
}

impl FileWriter for CloudFileWriter {
    fn output_zstd_compressed<R: Read>(
        output_dir: &Path,
        file: &mut R,
        archive_file_name: &str,
        compression_level: CompressionLevel,
    ) -> Result<(Sha1Checksum, FileSize), Box<dyn std::error::Error>> {
        // TODO: compress first to local temp file, then upload to cloud storage
        let system_temp_dir = std::env::temp_dir();
        let zstd_file_path = system_temp_dir
            .join(archive_file_name)
            .with_extension("zst");
        if let Some(parent) = zstd_file_path.parent() {
            create_dir_all(parent)?;
        }
        let zstd_file = File::create(zstd_file_path)?;
        let mut encoder = Encoder::new(zstd_file, compression_level.to_zstd_level())?;
        let mut buffer = [0u8; 8192]; // 8 KB buffer
        let mut hasher = Sha1::new();
        let mut size: u64 = 0;

        // ..

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // EOF
            }
            size += bytes_read as u64;
            hasher.update(&buffer[..bytes_read]);
            encoder.write_all(&buffer[..bytes_read])?;
        }
        encoder.finish()?;
        let checksum: GenericArray<u8, U20> = hasher.finalize();
        let checksum: Sha1Checksum = checksum.into();

        let region = Region::Custom {
            region: "eu-central-003".into(),
            endpoint: "s3.eu-central-003.backblazeb2.com".into(),
        };

        let credentials = Credentials::default()
            .map_err(|e| CloudStorageError::Other(format!("Credentials error: {e}")))?;

        let bucket = Bucket::new("efm-files", region, credentials)?.with_path_style();

        let key = format!("{}.zst", archive_file_name);

        //let mut encoder = Encoder::new(writer, compression_level.to_zstd_level())?;

        let mut hasher = Sha1::new();
        let mut size: u64 = 0;

        let upload_id = bucket
            .initiate_multipart_upload(key.as_str(), "application/zstd")
            .await?;

        let mut part_number = 1;
        let mut buffer = vec![0u8; 8 * 1024 * 1024]; // 8 MB buffer
        let mut parts = Vec::new();

        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break; // EOF
            }

            // Compress chunk
            let mut compressed_chunk = Vec::new();
            {
                let mut encoder = Encoder::new(&mut compressed_chunk, 3)?;
                encoder.write_all(&buffer[..bytes_read])?;
                encoder.finish()?;
            }
            // Upload part
            let part = bucket
                .put_multipart_chunk(
                    compressed_chunk,
                    key.as_str(),
                    part_number,
                    &upload_id.upload_id,
                    "application/zstd",
                )
                .await?;
            parts.push(part);
            part_number += 1;

            size += bytes_read as u64;
            hasher.update(&buffer[..bytes_read]);
        }
        //encoder.finish()?;
        bucket
            .complete_multipart_upload(key.as_str(), &upload_id.upload_id, parts)
            .await?;
        let checksum: GenericArray<u8, U20> = hasher.finalize();
        let checksum: Sha1Checksum = checksum.into();
        Ok((checksum, size))
    }
}
