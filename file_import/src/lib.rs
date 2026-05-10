pub mod file_import_ops;
pub mod file_outputter;
use core_types::{FileSize, FileType, ImportedFile, Sha1Checksum};
pub use file_import_ops::{FileImportOps, StdFileImportOps, mock};
use file_outputter::{CompressionLevel, output_zstd_compressed};
use file_system::fs_ops::{FsOps, StdFsOps};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
};
use tempfile::tempdir;
use utils::file_util::{self};
use zip::ZipArchive;

use uuid::Uuid;

#[derive(Debug, Clone)]
pub enum FileImportError {
    ZipError(String),
    FileIoError(String),
    SelectionMismatch(String),
}

/// Used for filtering files that will be imported.
#[derive(Debug)]
pub struct SelectedImportEntry {
    // Checksum for selected file entry.
    pub sha1_checksum: Sha1Checksum,
    // File name that will be used for storing the file entry.
    pub file_name: String,
}

#[derive(Debug)]
pub struct FileImportModel {
    pub file_path: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub file_type: FileType,
    pub selected_entries: HashMap<Sha1Checksum, SelectedImportEntry>,
}

impl Display for FileImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileImportError::ZipError(err) => write!(f, "Zip error: {}", err),
            FileImportError::FileIoError(err) => write!(f, "File IO error: {}", err),
            FileImportError::SelectionMismatch(err) => write!(f, "Selection mismatch: {}", err),
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
        tracing::info!(
            file_type = %file_import_model.file_type, 
            file_path = ?file_path,
            "Importing file");

        let is_zip = file_util::is_zip_file(file_path).map_err(|e| {
            FileImportError::FileIoError(format!("Failed checking if file is zip: {}", e))
        })?;

        if is_zip {
            let res = import_files_from_zip(
                file_path,
                &file_import_model.output_dir,
                &file_import_model.selected_entries,
                &file_import_model.file_type,
            )?;
            imported_files_map.extend(res);
        } else {
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
        archive_file_name: Some(archive_file_name.to_string()),
        sha1_checksum,
        file_size,
    };

    let mut file_name_to_checksum_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();
    file_name_to_checksum_map.insert(sha1_checksum, imported_file);

    Ok(file_name_to_checksum_map)
}

/// Reads the given zip file and imports only the selected checksum entries.
///
/// Each ZIP member is staged to a temporary directory, compressed there, and hashed while
/// staging. Only members whose SHA1 matches an entry in `file_entries` are persisted into the
/// final collection output directory.
///
/// # Arguments
///
/// * `file_path` - The path to the zip file.
/// * `output_dir` - The directory where the files will be extracted.
/// * `file_entries` - file entries to be imported from archive. Only these files will be processed
///
/// # Returns
///
/// A `Result` containing a hash map with imported files keyed by checksum, or an error if the
/// operation fails. The output file names will be provided in `file_entries`.
///
pub fn import_files_from_zip(
    file_path: &Path,
    output_dir: &Path,
    file_entries: &HashMap<Sha1Checksum, SelectedImportEntry>,
    file_type: &FileType,
) -> Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError> {
    tracing::info!(
        fila_path = ?file_path,
        output_dir = ?output_dir,
        file_type = %file_type,
        "Importing files from zip"
    );

    let file = File::open(file_path)
        .map_err(|e| FileImportError::FileIoError(format!("Failed opening file: {}", e)))?;
    let mut archive = ZipArchive::new(file)
        .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
    let mut file_name_to_checksum_map: HashMap<Sha1Checksum, ImportedFile> = HashMap::new();
    let temp_dir = tempdir().map_err(|e| {
        FileImportError::FileIoError(format!("Failed creating temporary directory: {}", e))
    })?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| FileImportError::ZipError(format!("Failed reading Zip file: {}", e)))?;
        if !file.is_file() {
            continue;
        }

        let archive_file_name = generate_archive_file_name();
        let staged_file_path = temp_dir
            .path()
            .join(&archive_file_name)
            .with_extension("zst");
        let (sha1_checksum, file_size) = output_zstd_compressed(
            temp_dir.path(),
            &mut file,
            &archive_file_name,
            get_compression_level(file_type),
        )
        .map_err(|e| {
            FileImportError::FileIoError(format!("Failed writing file to output directory: {}", e))
        })?;
        let fs_ops = StdFsOps;
        let Some(file_entry) = file_entries.get(&sha1_checksum) else {
            remove_staged_file(&fs_ops, &staged_file_path)?;
            continue;
        };

        if file_name_to_checksum_map.contains_key(&sha1_checksum) {
            remove_staged_file(&fs_ops, &staged_file_path)?;
            continue;
        }

        persist_staged_file(&fs_ops, &staged_file_path, output_dir, &archive_file_name)?;

        let imported_file = ImportedFile {
            original_file_name: file_entry.file_name.clone(),
            archive_file_name: Some(archive_file_name.to_string()),
            sha1_checksum,
            file_size,
        };

        file_name_to_checksum_map.insert(sha1_checksum, imported_file);
    }

    if !file_entries.is_empty() && file_name_to_checksum_map.is_empty() {
        return Err(FileImportError::SelectionMismatch(format!(
            "No ZIP members in '{}' matched the selected SHA1 entries",
            file_path.display()
        )));
    }

    Ok(file_name_to_checksum_map)
}

fn persist_staged_file(
    ops: &dyn FsOps,
    staged_file_path: &Path,
    output_dir: &Path,
    archive_file_name: &str,
) -> Result<(), FileImportError> {
    ops.create_dir_all(output_dir).map_err(|e| {
        FileImportError::FileIoError(format!("Failed creating output directory: {}", e))
    })?;
    let output_path = output_dir.join(archive_file_name).with_extension("zst");

    match ops.rename(staged_file_path, &output_path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::CrossesDevices => {
            ops.copy(staged_file_path, &output_path).map_err(|e| {
                FileImportError::FileIoError(format!(
                    "Failed copying staged file to output directory: {}",
                    e
                ))
            })?;

            remove_staged_file(ops, staged_file_path)?;
            Ok(())
        }
        Err(err) => Err(FileImportError::FileIoError(format!(
            "Failed moving staged file to output directory: {}",
            err
        ))),
    }
}

fn remove_staged_file(ops: &dyn FsOps, staged_file_path: &Path) -> Result<(), FileImportError> {
    if staged_file_path.exists() {
        ops.remove_file(staged_file_path).map_err(|e| {
            FileImportError::FileIoError(format!("Failed removing staged file: {}", e))
        })?;
    }
    Ok(())
}

// Import given file and store to interal file format.
// If file is zipped, import each file individually. If also single non zipped files individually.
// Checks file type, if file type is jpg or png,

fn generate_archive_file_name() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use std::{
        io::Write,
        sync::{Arc, Mutex},
    };

    use super::*;
    use file_system::fs_ops::FsOpsCall;
    use file_system::fs_ops::{FsOpsOutcome, MockFsOps, MockFsOpsState};
    use tempfile::tempdir;
    use utils::test_utils::get_sha1_and_size;
    use zip::write::FileOptions;

    const TEST_FILE_CONTENT: &str = "Hello, world!";
    const TEST_FILE_NAME: &str = "test_file";
    const TEST_ZIP_ARCHIVE_NAME: &str = "test.zip";

    #[test]
    fn test_import_files_from_zip_uses_sha1_selection_and_stored_file_name() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().to_path_buf();

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer
            .start_file("archive_member_name.bin", file_options)
            .unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT.as_bytes()).unwrap();
        zip_writer.finish().unwrap();
        let (checksum, size) = get_sha1_and_size(TEST_FILE_CONTENT);
        let mut selected_entries = HashMap::new();
        selected_entries.insert(
            checksum,
            SelectedImportEntry {
                sha1_checksum: checksum,
                file_name: TEST_FILE_NAME.to_string(),
            },
        );
        let result = import_files_from_zip(
            &zip_file_path,
            &output_path,
            &selected_entries,
            &FileType::Rom,
        );
        assert!(result.is_ok());
        let hash_map = result.unwrap();
        assert_eq!(hash_map.len(), 1);

        let imported_file = hash_map.get(&checksum).unwrap();
        assert_eq!(TEST_FILE_NAME, imported_file.original_file_name);
        assert!(imported_file.archive_file_name.is_some());
        assert_eq!(imported_file.sha1_checksum, checksum);
        assert_eq!(imported_file.file_size, size);
    }

    #[test]
    fn test_import_files_from_zip_ignores_unselected_members() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().to_path_buf();

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file("selected.bin", file_options).unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT.as_bytes()).unwrap();
        zip_writer.start_file("other.bin", file_options).unwrap();
        zip_writer.write_all(b"something else").unwrap();
        zip_writer.finish().unwrap();

        let (checksum, _) = get_sha1_and_size(TEST_FILE_CONTENT);
        let mut selected_entries = HashMap::new();
        selected_entries.insert(
            checksum,
            SelectedImportEntry {
                sha1_checksum: checksum,
                file_name: TEST_FILE_NAME.to_string(),
            },
        );

        let result = import_files_from_zip(
            &zip_file_path,
            &output_path,
            &selected_entries,
            &FileType::Rom,
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert!(result.contains_key(&checksum));
    }

    #[test]
    fn test_import_files_from_zip_returns_selection_mismatch_when_no_match_found() {
        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().to_path_buf();

        let zip_file_path = output_path.join(TEST_ZIP_ARCHIVE_NAME);
        let zip_file = File::create(&zip_file_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(zip_file);
        let file_options: FileOptions<'_, ()> = FileOptions::default();
        zip_writer.start_file(TEST_FILE_NAME, file_options).unwrap();
        zip_writer.write_all(TEST_FILE_CONTENT.as_bytes()).unwrap();
        zip_writer.finish().unwrap();

        let mut selected_entries = HashMap::new();
        selected_entries.insert(
            [9u8; 20],
            SelectedImportEntry {
                sha1_checksum: [9u8; 20],
                file_name: TEST_FILE_NAME.to_string(),
            },
        );

        let result = import_files_from_zip(
            &zip_file_path,
            &output_path,
            &selected_entries,
            &FileType::Rom,
        );

        assert!(matches!(
            result,
            Err(FileImportError::SelectionMismatch(message))
                if message.contains("matched the selected SHA1 entries")
        ));
    }

    #[test]
    fn test_persist_staged_file_when_create_dir_fails_return_error() {
        let fs_mock_state = Arc::new(Mutex::new(MockFsOpsState {
            outcome: FsOpsOutcome {
                create_dir_all_result: Some(Err(std::io::Error::other("create dir error"))),
                ..Default::default()
            },
            ..Default::default()
        }));
        let fs_ops = MockFsOps::new(Arc::clone(&fs_mock_state));
        let staged_file_path = Path::new("/temp/");
        let output_dir = Path::new("/output/");
        let archive_file_name = "archive_file_name";
        let res = persist_staged_file(&fs_ops, staged_file_path, output_dir, archive_file_name);
        assert!(res.is_err());
        let err = res.expect_err("Error expected");
        assert!(matches!(err, FileImportError::FileIoError(_)));

        let guard = fs_mock_state.lock().unwrap();
        assert_eq!(guard.calls.len(), 1);
        let call = &guard.calls[0];
        assert_eq!(
            *call,
            FsOpsCall::CreateDir {
                path: PathBuf::from("/output/")
            }
        );
    }

    #[test]
    fn test_persist_staged_file_when_rename_succeeds() {
        let fs_mock_state = Arc::new(Mutex::new(MockFsOpsState {
            outcome: FsOpsOutcome {
                create_dir_all_result: Some(Ok(())),
                rename_result: Some(Ok(())),
                ..Default::default()
            },
            ..Default::default()
        }));
        let fs_ops = MockFsOps::new(Arc::clone(&fs_mock_state));
        let staged_file_path = Path::new("/temp/");
        let output_dir = Path::new("/output/");
        let archive_file_name = "archive_file_name";
        let expected_output_path = output_dir.join(archive_file_name).with_extension("zst");

        let res = persist_staged_file(&fs_ops, staged_file_path, output_dir, archive_file_name);
        assert!(res.is_ok());

        let guard = fs_mock_state.lock().unwrap();
        assert_eq!(guard.calls.len(), 2);
        let call1 = &guard.calls[0];
        assert_eq!(
            *call1,
            FsOpsCall::CreateDir {
                path: PathBuf::from("/output/")
            }
        );
        let call2 = &guard.calls[1];
        assert_eq!(
            *call2,
            FsOpsCall::Rename {
                from: PathBuf::from(staged_file_path),
                to: expected_output_path.clone()
            }
        );
    }

    #[test]
    fn test_persist_staged_file_when_rename_fails_with_cross_device_copy_is_used_instead() {
        let fs_mock_state = Arc::new(Mutex::new(MockFsOpsState {
            outcome: FsOpsOutcome {
                create_dir_all_result: Some(Ok(())),
                rename_result: Some(Err(std::io::Error::new(
                    std::io::ErrorKind::CrossesDevices,
                    "cross-device link",
                ))),
                copy_result: Some(Ok(1234)),
                remove_result: Some(Ok(())),
            },
            ..Default::default()
        }));
        let fs_ops = MockFsOps::new(Arc::clone(&fs_mock_state));
        let staged_file_path = Path::new("/temp/");
        let output_dir = Path::new("/output/");
        let archive_file_name = "archive_file_name";
        let expected_output_path = output_dir.join(archive_file_name).with_extension("zst");

        let res = persist_staged_file(&fs_ops, staged_file_path, output_dir, archive_file_name);
        assert!(res.is_ok());

        let guard = fs_mock_state.lock().unwrap();
        assert_eq!(guard.calls.len(), 3);
        let call1 = &guard.calls[0];
        assert_eq!(
            *call1,
            FsOpsCall::CreateDir {
                path: PathBuf::from("/output/")
            }
        );
        let call2 = &guard.calls[1];
        assert_eq!(
            *call2,
            FsOpsCall::Rename {
                from: PathBuf::from(staged_file_path),
                to: expected_output_path.clone()
            }
        );
        let call3 = &guard.calls[2];
        assert_eq!(
            *call3,
            FsOpsCall::Copy {
                from: PathBuf::from(staged_file_path),
                to: expected_output_path
            },
        );
    }

    #[test]
    fn test_persist_staged_file_when_rename_fails_with_unexpected_error_return_error() {
        let fs_mock_state = Arc::new(Mutex::new(MockFsOpsState {
            outcome: FsOpsOutcome {
                create_dir_all_result: Some(Ok(())),
                rename_result: Some(Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "permission denied",
                ))),
                copy_result: Some(Ok(1234)),
                remove_result: Some(Ok(())),
            },
            ..Default::default()
        }));
        let fs_ops = MockFsOps::new(Arc::clone(&fs_mock_state));
        let staged_file_path = Path::new("/temp/");
        let output_dir = Path::new("/output/");
        let archive_file_name = "archive_file_name";
        let expected_output_path = output_dir.join(archive_file_name).with_extension("zst");

        let res = persist_staged_file(&fs_ops, staged_file_path, output_dir, archive_file_name);
        assert!(res.is_err());
        let err = res.expect_err("expected error");
        assert!(matches!(err, FileImportError::FileIoError(_)));

        let guard = fs_mock_state.lock().unwrap();
        assert_eq!(guard.calls.len(), 2);
        let call1 = &guard.calls[0];
        assert_eq!(
            *call1,
            FsOpsCall::CreateDir {
                path: PathBuf::from("/output/")
            }
        );
        let call2 = &guard.calls[1];
        assert_eq!(
            *call2,
            FsOpsCall::Rename {
                from: PathBuf::from(staged_file_path),
                to: expected_output_path.clone()
            }
        );
    }
}
