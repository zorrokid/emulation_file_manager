pub mod file_outputter;
use core_types::{FileSize, ImportedFile, Sha1Checksum};
pub use file_outputter::{CompressionMethod, FileOutputter};
use sha1::{
    digest::{consts::U20, generic_array::GenericArray},
    Digest, Sha1,
};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    io::Read,
    path::PathBuf,
};

use zip::ZipArchive;

#[derive(Debug, Clone)]
pub enum FileImportError {
    ZipError(String),
    FileIoError(String),
}

impl Display for FileImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileImportError::ZipError(err) => write!(f, "Zip error: {}", err),
            FileImportError::FileIoError(err) => write!(f, "File IO error: {}", err),
        }
    }
}

/// Reads the give zip file and imports the files listed in filter to the output directory in given compression method.
///
/// Calculate the checksum of each file in the zip archive and return a hash map of imported files with file names and their checksums.
///
/// # Arguments
///
/// * `file_path` - The path to the zip file.
/// * `output_dir` - The directory where the files will be extracted.
/// * `compression_type` - The compression method to use for the output files.
/// * `file_name_filter` - A hash set of file names to be imported from archive.
///
/// # Returns
///
/// A `Result` containing a hash map with file names and their checksums, or an error if the operation fails.
///
pub fn import_files_from_zip(
    file_path: PathBuf,
    output_dir: PathBuf,
    compression_type: CompressionMethod,
    file_name_filter: HashSet<String>,
) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
    let file = File::open(file_path)
        .map_err(|e| FileImportError::FileIoError(format!("Failed opening file: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
    let mut file_name_to_checksum_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
        if file.is_file() && file_name_filter.contains(file.name()) {
            let (sha1_checksum, file_size) = compression_type
                .output(&output_dir, &mut file)
                .map_err(|e| {
                    FileImportError::FileIoError(format!(
                        "Failed writing file to output directory: {}",
                        e
                    ))
                })?;
            let imported_file = ImportedFile {
                file_name: file.name().to_string(),
                sha1_checksum,
                file_size,
            };

            file_name_to_checksum_map.insert(sha1_checksum, imported_file);
        }
    }
    Ok(file_name_to_checksum_map)
}

/// Get the contents of a zip file.
///
/// # Arguments
///
/// * `file_path` - The path to the zip file.
///
/// # Returns
///
/// A `Result` containing a list of file names in the archive or an error if the operation fails.
pub fn read_zip_contents(file_path: PathBuf) -> Result<HashSet<String>, FileImportError> {
    let file = File::open(file_path)
        .map_err(|e| FileImportError::FileIoError(format!("Failed opening file: {}", e)))?;
    let archive = ZipArchive::new(file)
        .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
    let zip_contents = archive
        .file_names()
        .map(|name| name.to_string())
        .collect::<HashSet<_>>();

    Ok(zip_contents)
}

/// Get the contents of a zip file and calculate sha1 checksum and size for each file.
///
/// # Arguments
///
/// * `file_path` - The path to the zip file.
///
/// # Returns
///
/// A `Result` containing hash map from sha1 key to ImportFile with file name, sha1 checksum and size from files in the archive or an error if the operation fails.
pub fn read_zip_contents_with_checksums(
    file_path: PathBuf,
) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
    let file = File::open(file_path)
        .map_err(|e| FileImportError::FileIoError(format!("Failed opening file: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;

    let mut sha1_to_file_name_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
        if file.is_file() {
            let mut buffer = [0u8; 8192]; // 8 KB buffer
            let mut hasher = Sha1::new();
            let mut size: u64 = 0;
            loop {
                let bytes_read = file.read(&mut buffer).map_err(|e| {
                    FileImportError::FileIoError(format!("Failed reading file: {}", e))
                })?;
                if bytes_read == 0 {
                    break; // EOF
                }
                size += bytes_read as u64;
                hasher.update(&buffer[..bytes_read]);
            }
            let sha1_checksum: GenericArray<u8, U20> = hasher.finalize();
            let sha1_checksum: Sha1Checksum = sha1_checksum.into();
            let imported_file = ImportedFile {
                file_name: file.name().to_string(),
                sha1_checksum,
                file_size: size,
            };
            sha1_to_file_name_map.insert(sha1_checksum, imported_file);
        }
    }

    Ok(sha1_to_file_name_map)
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use tempfile::tempdir;
    use utils::test_utils::get_sha1_and_size;
    use zip::write::FileOptions;

    const TEST_FILE_CONTENT: &str = "Hello, world!";
    const TEST_FILE_NAME: &str = "test_file";
    const TEST_ZIP_ARCHIVE_NAME: &str = "test.zip";

    #[test]
    fn test_read_zip_file() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.into_path();
        let method = CompressionMethod::Zstd;

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(TEST_FILE_NAME, file_options).unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT.as_bytes()).unwrap();
        zip_writer.finish().unwrap();
        let mut file_name_filter = HashSet::new();
        file_name_filter.insert(TEST_FILE_NAME.to_string());
        let result = import_files_from_zip(zip_file_path, output_path, method, file_name_filter);
        let (checksum, size) = get_sha1_and_size(TEST_FILE_CONTENT);
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);

        assert_eq!(
            hash_map[&checksum],
            ImportedFile {
                file_name: TEST_FILE_NAME.to_string(),
                sha1_checksum: checksum,
                file_size: size,
            }
        );
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
        zip_writer.write_all(TEST_FILE_CONTENT.as_bytes()).unwrap();
        zip_writer.finish().unwrap();

        let result = read_zip_contents(zip_file_path);
        assert!(result.is_ok());
        let hash_set = result.unwrap();
        assert_eq!(hash_set.len(), 1);
        assert!(hash_set.contains(TEST_FILE_NAME));
    }

    #[test]
    fn test_read_zip_contents_with_checksums() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path();

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(TEST_FILE_NAME, file_options).unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT.as_bytes()).unwrap();
        zip_writer.finish().unwrap();

        let result = read_zip_contents_with_checksums(zip_file_path);
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);
        let (checksum, _) = get_sha1_and_size(TEST_FILE_CONTENT);
        assert!(hash_map.contains_key(&checksum));
        let expected_file = ImportedFile {
            file_name: TEST_FILE_NAME.to_string(),
            sha1_checksum: checksum,
            file_size: TEST_FILE_CONTENT.len() as u64,
        };
        assert_eq!(hash_map[&checksum], expected_file);
    }
}
