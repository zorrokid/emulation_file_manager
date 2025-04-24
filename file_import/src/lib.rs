pub mod file_outputter;
pub use file_outputter::{CompressionMethod, FileOutputter};
use sha1::{Digest, Sha1};
use std::{collections::HashMap, fs::File, io::Read, path::Path};

use zip::ZipArchive;

/// Reads the give zip file and imports the files listed in file_name_checksum_filter to the output directory in given compression method.
///
/// Calculate the checksum of each file in the zip archive, compare the checksum to give checksum
/// in file_name_checksum_filter and also return a hash map of imported files with file names and their checksums.
///
/// # Arguments
///
/// * `file_path` - The path to the zip file.
/// * `output_dir` - The directory where the files will be extracted.
/// * `compression_type` - The compression method to use for the output files.
/// * `file_name_checksum_filter` - A hash map of files to be imported from archive containing file names and their expected checksums.
///
/// # Returns
///
/// A `Result` containing a hash map with file names and their checksums, or an error if the operation fails.
///
pub fn import_files_from_zip(
    file_path: &str,
    output_dir: &str,
    compression_type: CompressionMethod,
    file_name_checksum_filter: HashMap<String, String>,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut file_name_to_checksum_map = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_file() && file_name_checksum_filter.contains_key(file.name()) {
            let expected_checksum = file_name_checksum_filter
                .get(file.name())
                .ok_or_else(|| format!("Checksum not found for file: {}", file.name()))?;
            let output_path = Path::new(output_dir);
            let checksum = compression_type.output(output_path, &mut file)?;
            if checksum != *expected_checksum {
                return Err(format!(
                    "Checksum mismatch for file: {}. Expected: {}, Got: {}",
                    file.name(),
                    expected_checksum,
                    checksum
                )
                .into());
            }
            file_name_to_checksum_map.insert(file.name().to_string(), checksum);
        }
    }
    Ok(file_name_to_checksum_map)
}

/// Reads the contents of a zip file and calculates the SHA-1 checksum for each file.
///
/// # Arguments
///
/// * `file_path` - The path to the zip file.
///
/// # Returns
///
/// A `Result` containing a hash map with file names and their SHA-1 checksums, or an error if the operation fails.
pub fn read_zip_contents(
    file_path: &Path,
) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut file_name_to_checksum_map = HashMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_file() {
            let mut buffer = [0u8; 8192]; // 8 KB buffer
            let mut hasher = Sha1::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break; // EOF
                }
                hasher.update(&buffer[..bytes_read]);
            }
            let checksum = hasher.finalize();
            let checksum_as_string = format!("{:x}", checksum);
            file_name_to_checksum_map.insert(file.name().to_string(), checksum_as_string);
        }
    }
    Ok(file_name_to_checksum_map)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use tempfile::tempdir;
    use zip::write::FileOptions;

    const TEST_FILE_CONTENT: &[u8] = b"Hello, world!";
    const TEST_FILE_CONTENT_SHA1: &str = "943a702d06f34599aee1f8da8ef9f7296031d699";
    const TEST_FILE_NAME: &str = "test_file";
    const TEST_ZIP_ARCHIVE_NAME: &str = "test.zip";

    #[test]
    fn test_read_zip_file() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();
        let method = CompressionMethod::Zstd;

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(TEST_FILE_NAME, file_options).unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT).unwrap();
        zip_writer.finish().unwrap();
        let mut file_name_checksum_filter = HashMap::new();
        file_name_checksum_filter.insert(
            TEST_FILE_NAME.to_string(),
            TEST_FILE_CONTENT_SHA1.to_string(),
        );
        let result = import_files_from_zip(
            zip_file_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
            method,
            file_name_checksum_filter,
        );
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);
        assert_eq!(hash_map[TEST_FILE_NAME], TEST_FILE_CONTENT_SHA1);
    }

    #[test]
    fn test_read_zip_contents() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(TEST_FILE_NAME, file_options).unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT).unwrap();
        zip_writer.finish().unwrap();

        let result = read_zip_contents(zip_file_path.as_path());
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);
        assert_eq!(hash_map[TEST_FILE_NAME], TEST_FILE_CONTENT_SHA1);
    }
}
