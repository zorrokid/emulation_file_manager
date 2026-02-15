use std::path::Path;

use utils::file_util::is_zip_file;

use crate::{
    FileMetadataError, FileMetadataReader, FileType, SingleFileMetadataReader,
    ZipFileMetadataReader,
};

/// Type alias for factory function
pub type ReaderFactoryFn = dyn Fn(&Path) -> Result<Box<dyn FileMetadataReader>, FileMetadataError>;

/// Create appropriate reader based on file type
pub fn create_metadata_reader(
    path: &Path,
) -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
    match detect_file_type(path)? {
        FileType::Single => Ok(Box::new(SingleFileMetadataReader::new(path)?)),
        FileType::Zip => Ok(Box::new(ZipFileMetadataReader::new(path)?)),
        // TODO: FileType::7z, etc.
    }
}

// TODO: this could be extended to detect more file types in the future and moved to a
// separate module or crate if needed.
fn detect_file_type(path: &Path) -> Result<FileType, FileMetadataError> {
    match path.extension().and_then(|s| s.to_str()) {
        Some("zip") => match is_zip_file(path) {
            Ok(true) => Ok(FileType::Zip),
            Ok(false) => Err(FileMetadataError::UnsupportedFormat(path.to_path_buf())),
            Err(e) => Err(FileMetadataError::FileIoError {
                path: path.to_path_buf(),
                message: format!("Failed reading file: {}", e),
            }),
        },
        _ => Ok(FileType::Single),
    }
}

#[cfg(test)]
mod tests {
    use crate::FileMetadataError;

    use super::*;
    use core_types::sha1_bytes_to_hex_string;

    use std::path::Path;
    #[test]
    fn test_create_metadata_reader_single_file() {
        let test_file_path = Path::new("example-data/one_byte_255.bin");
        let reader = create_metadata_reader(test_file_path).unwrap();
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
    fn test_create_metadata_reader_zip_file() {
        let test_file_path = Path::new("example-data/one_byte_255.zip");
        let reader = create_metadata_reader(test_file_path).unwrap();
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
    fn test_create_metadata_reader_invalid_zip() {
        let test_file_path = Path::new("example-data/invalid.zip");
        let result = create_metadata_reader(test_file_path);
        assert!(result.is_err());
        let error = result.err().unwrap();
        println!("Error: {:?}", error);
        match error {
            FileMetadataError::UnsupportedFormat(path) => {
                assert_eq!(path, test_file_path);
            }
            _ => panic!("Expected UnsupportedFormat error"),
        }
    }
}
