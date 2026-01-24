pub mod file_import_ops;
pub mod file_outputter;
use core_types::{FileSize, FileType, ImportedFile, Sha1Checksum};
pub use file_import_ops::{FileImportOps, StdFileImportOps, mock};
use file_outputter::{CompressionLevel, output_zstd_compressed};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
};
use utils::file_util::{self};
use zip::ZipArchive;

use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum FileImportError {
    ZipError(String),
    FileIoError(String),
}

#[derive(Debug)]
pub struct FileImportModel {
    pub file_path: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub file_type: FileType,
    pub new_files_file_name_filter: HashSet<String>,
}

impl Display for FileImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileImportError::ZipError(err) => write!(f, "Zip error: {}", err),
            FileImportError::FileIoError(err) => write!(f, "File IO error: {}", err),
        }
    }
}

pub fn get_compression_level(file_type: &FileType) -> CompressionLevel {
    match file_type {
        FileType::Rom | FileType::DiskImage | FileType::TapeImage | FileType::MemorySnapshot => {
            CompressionLevel::Good
        }
        FileType::Screenshot
        | FileType::Manual
        | FileType::CoverScan
        | FileType::TitleScreen
        | FileType::LoadingScreen => CompressionLevel::Fast,
        _ => CompressionLevel::Default,
    }
}

pub fn import(
    file_import_model: &FileImportModel,
) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
    let mut imported_files_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();
    for file_path in &file_import_model.file_path {
        println!(
            "Importing file with file type: {} from path: {} to output directory: {} with filter: {:?}",
            //file_import_model.file_name,
            file_import_model.file_type,
            file_path.display(),
            file_import_model.output_dir.display(),
            file_import_model.new_files_file_name_filter
        );

        let is_zip = file_util::is_zip_file(file_path).map_err(|e| {
            FileImportError::FileIoError(format!("Failed checking if file is zip: {}", e))
        })?;

        if is_zip {
            let res = import_files_from_zip(
                file_path,
                &file_import_model.output_dir,
                &file_import_model.new_files_file_name_filter,
                &file_import_model.file_type,
            )?;
            imported_files_map.extend(res);
        } else {
            println!(
                "Importing file with file type: {}",
                file_import_model.file_type
            );
            let res = import_file(
                file_path,
                &file_import_model.output_dir,
                &file_import_model.file_type,
            )?;
            imported_files_map.extend(res);
        }
    }
    Ok(imported_files_map)
}

/// Import single-non zipped file.
pub fn import_file(
    file_path: &Path,
    output_dir: &Path,
    file_type: &FileType,
) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
    let mut file = File::open(file_path)
        .map_err(|e| FileImportError::FileIoError(format!("Failed opening file: {}", e)))?;
    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| FileImportError::FileIoError("Failed to get file name".to_string()))?;
    let archive_file_name = generate_archive_file_name();
    let (sha1_checksum, file_size) = output_zstd_compressed(
        output_dir,
        &mut file,
        &archive_file_name,
        get_compression_level(file_type),
    )
    .map_err(|e| {
        FileImportError::FileIoError(format!("Failed writing file to output directory: {}", e))
    })?;
    let imported_file = ImportedFile {
        original_file_name: file_name.to_string(),
        archive_file_name: archive_file_name.to_string(),
        sha1_checksum,
        file_size,
    };

    let mut file_name_to_checksum_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();
    file_name_to_checksum_map.insert(sha1_checksum, imported_file);

    Ok(file_name_to_checksum_map)
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
/// * `file_name_filter` - A hash set of file names to be imported from archive. Only these files will be processed
///
/// # Returns
///
/// A `Result` containing a hash map with file names and their checksums, or an error if the operation fails.
///
pub fn import_files_from_zip(
    file_path: &Path,
    output_dir: &Path,
    file_name_filter: &HashSet<String>,
    file_type: &FileType,
) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
    println!(
        "Importing files from zip: {} to output directory: {} with file filter: {:?} and file type {}.",
        file_path.display(),
        output_dir.display(),
        file_name_filter,
        file_type
    );

    let file = File::open(file_path)
        .map_err(|e| FileImportError::FileIoError(format!("Failed opening file: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
    let mut file_name_to_checksum_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
        let archive_file_name = generate_archive_file_name();
        if file.is_file() && file_name_filter.contains(file.name()) {
            let (sha1_checksum, file_size) = output_zstd_compressed(
                output_dir,
                &mut file,
                &archive_file_name,
                get_compression_level(file_type),
            )
            .map_err(|e| {
                FileImportError::FileIoError(format!(
                    "Failed writing file to output directory: {}",
                    e
                ))
            })?;
            let imported_file = ImportedFile {
                original_file_name: file.name().to_string(),
                archive_file_name: archive_file_name.to_string(),
                sha1_checksum,
                file_size,
            };

            file_name_to_checksum_map.insert(sha1_checksum, imported_file);
        }
    }
    Ok(file_name_to_checksum_map)
}

// Import given file and store to interal file format.
// If file is zipped, import each file individually. If also single non zipped files individually.
// Checks file type, if file type is jpg or png,

fn generate_archive_file_name() -> String {
    Uuid::new_v4().to_string()
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
    fn test_import_files_from_zip() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.into_path();

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(TEST_FILE_NAME, file_options).unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT.as_bytes()).unwrap();
        zip_writer.finish().unwrap();
        let mut file_name_filter = HashSet::new();
        file_name_filter.insert(TEST_FILE_NAME.to_string());
        let result = import_files_from_zip(
            &zip_file_path,
            &output_path,
            &file_name_filter,
            &FileType::Rom,
        );
        let (checksum, size) = get_sha1_and_size(TEST_FILE_CONTENT);
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);

        let imported_file = hash_map.get(&checksum).unwrap();
        assert_eq!(TEST_FILE_NAME, imported_file.original_file_name);
        assert!(!imported_file.archive_file_name.is_empty());
        assert_eq!(imported_file.sha1_checksum, checksum);
        assert_eq!(imported_file.file_size, size);
    }
}
