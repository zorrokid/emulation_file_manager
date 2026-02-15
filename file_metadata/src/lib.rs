pub mod file_metadata_ops;
pub mod reader_factory;

use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use core_types::{ReadFile, Sha1Checksum};
use sha1::{
    Digest, Sha1,
    digest::{consts::U20, generic_array::GenericArray},
};

use utils::file_util::{self};
use zip::ZipArchive;

/// Supported file types for metadata extraction
pub enum FileType {
    Single,
    Zip,
    // TODO 7z,
}

#[derive(Debug, thiserror::Error)]
pub enum FileMetadataError {
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Checksum error for {path}: {message}")]
    ChecksumError { path: PathBuf, message: String },

    #[error("File I/O error for {path}: {message}")]
    FileIoError { path: PathBuf, message: String },

    #[error("Zip error for {path}: {message}")]
    ZipError { path: PathBuf, message: String },

    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(PathBuf),

    #[error("General error for {path}: {message}")]
    GeneralError { path: PathBuf, message: String },
}

/// Trait for reading file metadata from various sources
pub trait FileMetadataReader: Send + Sync {
    /// Read metadata for all files in this source
    ///
    /// Returns a Vec since archives can contain multiple files.
    /// Single files return a Vec with one element for consistent interface.
    ///
    /// Note: This is a blocking operation. Checksumming large files may take time.
    /// Future enhancement: async version with progress callbacks.
    fn read_metadata(&self) -> Result<Vec<ReadFile>, FileMetadataError>;
}

/// Read single file meta data
pub struct SingleFileMetadataReader {
    path: PathBuf,
}

impl SingleFileMetadataReader {
    pub fn new(path: &Path) -> Result<Self, FileMetadataError> {
        if !path.exists() {
            return Err(FileMetadataError::FileNotFound(path.to_path_buf()));
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl FileMetadataReader for SingleFileMetadataReader {
    fn read_metadata(&self) -> Result<Vec<ReadFile>, FileMetadataError> {
        let read_files = read_file_checksum(&self.path)?;
        Ok(read_files.into_values().collect())
    }
}

fn read_file_checksum(
    file_path: &PathBuf,
) -> Result<HashMap<Sha1Checksum, ReadFile>, FileMetadataError> {
    let sha1 = file_util::get_file_sha1(file_path);
    match sha1 {
        Ok(checksum) => {
            let file_size = std::fs::metadata(file_path)
                .map_err(|e| FileMetadataError::FileIoError {
                    path: file_path.clone(),
                    message: format!("Failed to get file size: {}", e),
                })?
                .len();
            let read_file = ReadFile {
                file_name: file_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                sha1_checksum: checksum,
                file_size,
            };
            let mut map = HashMap::new();
            map.insert(checksum, read_file);
            Ok(map)
        }
        Err(e) => Err(FileMetadataError::FileIoError {
            path: file_path.clone(),
            message: format!("Failed calculating SHA1 checksum: {}", e),
        }),
    }
}

pub struct ZipFileMetadataReader {
    path: PathBuf,
}

impl ZipFileMetadataReader {
    pub fn new(path: &Path) -> Result<Self, FileMetadataError> {
        if !path.exists() {
            return Err(FileMetadataError::FileNotFound(path.to_path_buf()));
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl FileMetadataReader for ZipFileMetadataReader {
    fn read_metadata(&self) -> Result<Vec<ReadFile>, FileMetadataError> {
        let entries = read_zip_contents_with_checksums(&self.path)?;
        Ok(entries.into_values().collect())
    }
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
fn read_zip_contents_with_checksums(
    file_path: &PathBuf,
) -> Result<HashMap<Sha1Checksum, ReadFile>, FileMetadataError> {
    let file = File::open(file_path).map_err(|e| FileMetadataError::FileIoError {
        path: file_path.clone(),
        message: format!("Failed opening file: {}", e),
    })?;
    let mut archive = ZipArchive::new(file).map_err(|e| FileMetadataError::ZipError {
        path: file_path.clone(),
        message: format!("Failed reading Zip file: {}", e),
    })?;

    let mut sha1_to_file_name_map: HashMap<Sha1Checksum, ReadFile> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| FileMetadataError::ZipError {
                path: file_path.clone(),
                message: format!("Failed reading Zip file: {}", e),
            })?;
        if file.is_file() {
            let mut buffer = [0u8; 8192]; // 8 KB buffer
            let mut hasher = Sha1::new();
            let mut size: u64 = 0;
            loop {
                let bytes_read =
                    file.read(&mut buffer)
                        .map_err(|e| FileMetadataError::FileIoError {
                            path: file_path.clone(),
                            message: format!("Failed reading file: {}", e),
                        })?;
                if bytes_read == 0 {
                    break; // EOF
                }
                size += bytes_read as u64;
                hasher.update(&buffer[..bytes_read]);
            }
            let sha1_checksum: GenericArray<u8, U20> = hasher.finalize();
            let sha1_checksum: Sha1Checksum = sha1_checksum.into();
            let read_file = ReadFile {
                file_name: file.name().to_string(),
                sha1_checksum,
                file_size: size,
            };
            sha1_to_file_name_map.insert(sha1_checksum, read_file);
        }
    }

    Ok(sha1_to_file_name_map)
}

#[derive(Clone)]
pub struct MockFileMetadataReader {
    pub metadata: Vec<ReadFile>,
}

pub fn create_mock_factory(
    reader: MockFileMetadataReader,
) -> impl Fn(&Path) -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
    move |_path: &Path| Ok(Box::new(reader.clone()))
}

impl FileMetadataReader for MockFileMetadataReader {
    fn read_metadata(&self) -> Result<Vec<ReadFile>, FileMetadataError> {
        Ok(self.metadata.clone())
    }
}

#[cfg(test)]
mod tests {

    use core_types::sha1_bytes_to_hex_string;
    use tempfile::tempdir;
    use utils::test_utils::get_sha1_and_size;
    use zip::write::FileOptions;

    use super::*;
    use std::{io::Write, path::Path};

    #[test]
    fn test_single_file_metadata_reader() {
        let test_file_path = Path::new("example-data/one_byte_255.bin");
        let reader = SingleFileMetadataReader::new(test_file_path).unwrap();
        let metadata = reader.read_metadata().unwrap();
        assert_eq!(metadata.len(), 1);
        let read_file = &metadata[0];
        assert_eq!(read_file.file_name, "one_byte_255.bin");

        assert_eq!(read_file.file_size, 1);
        assert_eq!(
            sha1_bytes_to_hex_string(&read_file.sha1_checksum),
            "85e53271e14006f0265921d02d4d736cdc580b0b"
        );
    }

    #[test]
    fn test_single_file_metadata_reader_empty_file() {
        let test_file_path = Path::new("example-data/empty.bin");
        let reader = SingleFileMetadataReader::new(test_file_path).unwrap();
        let metadata = reader.read_metadata().unwrap();
        assert_eq!(metadata.len(), 1);
        let read_file = &metadata[0];
        assert_eq!(read_file.file_name, "empty.bin");

        assert_eq!(read_file.file_size, 0);
        assert_eq!(
            sha1_bytes_to_hex_string(&read_file.sha1_checksum),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }

    #[test]
    fn test_single_file_metadata_reader_nonexistent_file() {
        let test_file_path = Path::new("example-data/nonexistent_file.bin");
        let result = SingleFileMetadataReader::new(test_file_path);
        assert!(result.is_err());
        match result.err().unwrap() {
            FileMetadataError::FileNotFound(path) => {
                assert_eq!(path, test_file_path);
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_zip_file_metadata_reader_single_file_zip() {
        let test_file_path = Path::new("example-data/one_byte_255.zip");
        let reader = ZipFileMetadataReader::new(test_file_path).unwrap();
        let metadata = reader.read_metadata().unwrap();
        assert_eq!(metadata.len(), 1);
        let read_file = &metadata[0];
        assert_eq!(read_file.file_name, "one_byte_255.bin");
        assert_eq!(read_file.file_size, 1);
        assert_eq!(
            sha1_bytes_to_hex_string(&read_file.sha1_checksum),
            "85e53271e14006f0265921d02d4d736cdc580b0b"
        );
    }

    #[test]
    fn test_zip_file_metadata_reader_multi_file_zip() {
        let test_file_path = Path::new("example-data/multiple_files.zip");
        let reader = ZipFileMetadataReader::new(test_file_path).unwrap();
        let metadata = reader.read_metadata().unwrap();
        assert_eq!(metadata.len(), 2);
        let one_byte_255_file = metadata
            .iter()
            .find(|f| f.file_name == "one_byte_255.bin")
            .unwrap();
        assert_eq!(one_byte_255_file.file_size, 1);
        assert_eq!(
            sha1_bytes_to_hex_string(&one_byte_255_file.sha1_checksum),
            "85e53271e14006f0265921d02d4d736cdc580b0b"
        );

        let empty_file = metadata
            .iter()
            .find(|f| f.file_name == "empty.bin")
            .unwrap();
        assert_eq!(empty_file.file_size, 0);
        assert_eq!(
            sha1_bytes_to_hex_string(&empty_file.sha1_checksum),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
    }

    #[test]
    fn test_zip_file_metadata_reader_nonexistent_file() {
        let test_file_path = Path::new("example-data/nonexistent_file.zip");
        let result = ZipFileMetadataReader::new(test_file_path);
        assert!(result.is_err());
        match result.err().unwrap() {
            FileMetadataError::FileNotFound(path) => {
                assert_eq!(path, test_file_path);
            }
            _ => panic!("Expected FileNotFound error"),
        }
    }

    #[test]
    fn test_zip_file_metadata_reader_invalid_zip() {
        let test_file_path = Path::new("example-data/invalid.zip");
        let reader = ZipFileMetadataReader::new(test_file_path).unwrap();
        let result = reader.read_metadata();
        assert!(result.is_err());
        match result.err().unwrap() {
            FileMetadataError::ZipError { path, message: _ } => {
                assert_eq!(path, test_file_path);
            }
            _ => panic!("Expected ZipError"),
        }
    }

    #[test]
    fn test_mock_file_metadata_reader() {
        let mock_metadata = vec![
            ReadFile {
                file_name: "file1.bin".to_string(),
                sha1_checksum: [0u8; 20],
                file_size: 123,
            },
            ReadFile {
                file_name: "file2.bin".to_string(),
                sha1_checksum: [1u8; 20],
                file_size: 456,
            },
        ];
        let mock_reader = MockFileMetadataReader {
            metadata: mock_metadata.clone(),
        };
        let metadata = mock_reader.read_metadata().unwrap();
        assert_eq!(metadata, mock_metadata);
    }

    #[test]
    fn test_with_mock_factory() {
        let mock_metadata = vec![ReadFile {
            file_name: "mock_file.bin".to_string(),
            sha1_checksum: [2u8; 20],
            file_size: 789,
        }];
        let mock_reader = MockFileMetadataReader {
            metadata: mock_metadata.clone(),
        };
        let factory = create_mock_factory(mock_reader);
        let test_path = Path::new("any_path.bin");
        let reader = factory(test_path).unwrap();
        let metadata = reader.read_metadata().unwrap();
        assert_eq!(metadata, mock_metadata);
    }

    const TEST_ZIP_ARCHIVE_NAME: &str = "test.zip";
    const TEST_FILE_NAME: &str = "test_file";
    const TEST_FILE_CONTENT: &str = "Hello, world!";

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

        let result = read_zip_contents_with_checksums(&zip_file_path);
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);
        let (checksum, _) = get_sha1_and_size(TEST_FILE_CONTENT);
        assert!(hash_map.contains_key(&checksum));
        let expected_file = ReadFile {
            file_name: TEST_FILE_NAME.to_string(),
            sha1_checksum: checksum,
            file_size: TEST_FILE_CONTENT.len() as u64,
        };
        assert_eq!(hash_map[&checksum], expected_file);
    }
}
