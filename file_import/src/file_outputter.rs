use sha1::{
    digest::{consts::U20, generic_array::GenericArray},
    Digest, Sha1,
};
use std::{
    fs::{create_dir_all, File},
    io::{Read, Write},
    path::Path,
    str::FromStr,
};
use zstd::Encoder;

use crate::{FileSize, Sha1Checksum};

#[derive(Clone, Debug, PartialEq)]
pub enum CompressionMethod {
    Zstd,
    None,
}

impl FromStr for CompressionMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "zstd" => Ok(CompressionMethod::Zstd),
            "none" => Ok(CompressionMethod::None),
            _ => Err(format!("Invalid compression method: {}", s)),
        }
    }
}

pub trait FileOutputter {
    fn output<R: Read>(
        &self,
        output_dir: &Path,
        file: &mut R,
        archive_file_name: &str,
    ) -> Result<(Sha1Checksum, FileSize), Box<dyn std::error::Error>>;
}

impl FileOutputter for CompressionMethod {
    fn output<R: Read>(
        &self,
        output_path: &Path,
        file: &mut R,
        archive_file_name: &str,
    ) -> Result<(Sha1Checksum, FileSize), Box<dyn std::error::Error>> {
        match self {
            CompressionMethod::Zstd => output_zstd_compressed(output_path, file, archive_file_name),
            CompressionMethod::None => {
                output_without_compression(output_path, file, archive_file_name)
            }
        }
    }
}

fn output_without_compression<R: Read>(
    output_dir: &Path,
    file: &mut R,
    archive_file_name: &str,
) -> Result<(Sha1Checksum, FileSize), Box<dyn std::error::Error>> {
    if let Some(parent) = output_dir.parent() {
        create_dir_all(parent)?;
    }
    let mut output_file = File::create(output_dir.join(archive_file_name))?;
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
        output_file.write_all(&buffer[..bytes_read])?;
    }
    let checksum: GenericArray<u8, U20> = hasher.finalize();
    let checksum: Sha1Checksum = checksum.into();
    Ok((checksum, size))
}

fn output_zstd_compressed<R: Read>(
    output_dir: &Path,
    file: &mut R,
    archive_file_name: &str,
) -> Result<(Sha1Checksum, FileSize), Box<dyn std::error::Error>> {
    let zstd_file_path = output_dir.join(archive_file_name).with_extension("zst");
    if let Some(parent) = zstd_file_path.parent() {
        create_dir_all(parent)?;
    }
    let zstd_file = File::create(zstd_file_path)?;
    let mut encoder = Encoder::new(zstd_file, 0)?;
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

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;
    use utils::test_utils::get_sha1_and_size;
    use zip::write::FileOptions;

    use super::*;

    const TEST_FILE_NAME: &str = "test_file";
    const TEST_ZIP_ARCHIVE_NAME: &str = "test_archive";
    const TEST_FILE_CONTENT: &str = "Hello, world!";
    const TEST_ARCHIVE_FILE_NAME: &str = "test_123";

    fn create_test_zip_file(
        output_path: &Path,
        file_name: &str,
        buffer: &[u8],
    ) -> Result<File, Box<dyn std::error::Error>> {
        let zip_file = File::create(output_path)?;
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(file_name, file_options)?;
        zip_writer.write_all(buffer)?;
        zip_writer.finish()?;
        let file = File::open(output_path).expect("Failed to open zip file");
        Ok(file)
    }

    #[test]
    fn test_output_without_compression() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let buffer = TEST_FILE_CONTENT.as_bytes();

        let zip_output_path = output_path
            .join(TEST_ZIP_ARCHIVE_NAME)
            .with_extension("zip");
        let file = create_test_zip_file(&zip_output_path, TEST_FILE_NAME, buffer).unwrap();
        let mut zip_archive = zip::ZipArchive::new(file).expect("Failed to read zip file");
        let mut zip_file = zip_archive
            .by_name(TEST_FILE_NAME)
            .expect("Failed to find file in zip archive");

        let method = CompressionMethod::None;

        let (expected_checksum, expected_size) = get_sha1_and_size(TEST_FILE_CONTENT);

        let (checksum, size) = method
            .output(output_path, &mut zip_file, TEST_ARCHIVE_FILE_NAME)
            .expect("Failed to write file");
        assert_eq!(checksum, expected_checksum);
        assert_eq!(size, expected_size);

        let output_data =
            fs::read(output_path.join(TEST_ARCHIVE_FILE_NAME)).expect("Failed to read file");
        assert_eq!(output_data, buffer);
    }

    #[test]
    fn test_output_zstd_compressed() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let method = CompressionMethod::Zstd;
        let file_content_buffer = TEST_FILE_CONTENT.as_bytes();

        let zip_output_path = output_path
            .join(TEST_ZIP_ARCHIVE_NAME)
            .with_extension("zip");
        let file =
            create_test_zip_file(&zip_output_path, TEST_FILE_NAME, file_content_buffer).unwrap();
        let mut zip_archive = zip::ZipArchive::new(file).expect("Failed to read zip file");
        let mut zip_file = zip_archive
            .by_name(TEST_FILE_NAME)
            .expect("Failed to find file in zip archive");

        let (expected_checksum, expected_size) = get_sha1_and_size(TEST_FILE_CONTENT);

        let (checksum, size) = method
            .output(output_path, &mut zip_file, TEST_ARCHIVE_FILE_NAME)
            .expect("Failed to write file");
        assert_eq!(checksum, expected_checksum);
        assert_eq!(size, expected_size);

        let output_data = fs::read(
            output_path
                .join(TEST_ARCHIVE_FILE_NAME)
                .with_extension("zst"),
        )
        .expect("Failed to read file");
        assert!(!output_data.is_empty());
    }
}
