use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use core_types::{FileSize, FileType, Sha1Checksum};
use file_import::FileImportModel;

#[derive(Debug, Clone)]
pub struct FileImportMetadata {
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub is_zip_archive: bool,
}

/// Content of a file to be imported. If there is already an existing file with the same
/// checksum, the existing file info will be provided.
#[derive(Debug, Clone)]
pub struct ImportFileContent {
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,

    pub existing_file_info_id: Option<i64>,
    pub existing_archive_file_name: Option<String>,
}

/// Single file import source model including path and content info.
#[derive(Debug, Clone)]
pub struct FileImportSource {
    /// Source path to the file to be imported (e.g., zip archive)
    pub path: PathBuf,
    /// Mapping of SHA1 checksum to file content info
    pub content: HashMap<Sha1Checksum, ImportFileContent>,
}

#[derive(Debug)]
pub struct FileImportData {
    pub output_dir: PathBuf,
    pub file_type: FileType,

    /// These are selected files from the file set source files that should be imported.
    /// For example, certain files from a zip archive can be selected separately.
    /// These span over all import files.
    pub selected_files: Vec<Sha1Checksum>,

    /// It is possible to create a file set from multiple sets of source import files.
    /// From this collection of files, the ones that are in selected_files and are new files, will be imported.
    /// If the files are existing files, they will be just linked to the file set.
    pub import_files: Vec<FileImportSource>,
}

impl From<&FileImportData> for FileImportModel {
    fn from(val: &FileImportData) -> Self {
        FileImportModel {
            file_path: val
                .import_files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<PathBuf>>(),
            output_dir: val.output_dir.clone(),
            file_type: val.file_type,
            new_files_file_name_filter: val.get_new_selected_file_names(),
        }
    }
}

impl FileImportData {
    pub fn get_new_selected_file_names(&self) -> HashSet<String> {
        self.import_files
            .iter()
            .flat_map(|file| {
                file.content
                    .iter()
                    .filter_map(|(sha1_checksum, import_content)| {
                        if self.selected_files.contains(sha1_checksum)
                            && import_content.existing_file_info_id.is_none()
                        {
                            Some(import_content.file_name.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect::<HashSet<String>>()
    }

    pub fn is_new_files_to_be_imported(&self) -> bool {
        !self.get_new_selected_file_names().is_empty()
    }
}

#[derive(Debug)]
pub struct FileImportPrepareResult {
    pub import_model: FileImportSource,
    pub import_metadata: FileImportMetadata,
}

#[derive(Debug)]
pub struct FileSetImportModel {
    pub import_files: Vec<FileImportSource>,
    pub selected_files: Vec<Sha1Checksum>,
    pub system_ids: Vec<i64>,
    pub source: String,
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub file_type: FileType,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_file_import_data(
        selected_files: Vec<Sha1Checksum>,
        import_files: Vec<FileImportSource>,
    ) -> FileImportData {
        FileImportData {
            file_type: FileType::Rom,
            selected_files,
            output_dir: PathBuf::from("/imported/files"),
            import_files,
        }
    }

    #[test]
    fn test_get_new_selected_file_names_empty() {
        let file_import_data = create_file_import_data(vec![], vec![]);
        let result = file_import_data.get_new_selected_file_names();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_new_selected_file_names_with_some_new_files() {
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];
        let checksum3: Sha1Checksum = [3u8; 20];

        let mut content = HashMap::new();
        content.insert(
            checksum1,
            ImportFileContent {
                file_name: "game1.rom".to_string(),
                sha1_checksum: checksum1,
                file_size: 1024,
                existing_file_info_id: None,
                existing_archive_file_name: None,
            },
        );
        content.insert(
            checksum2,
            ImportFileContent {
                file_name: "game2.rom".to_string(),
                sha1_checksum: checksum2,
                file_size: 2048,
                existing_file_info_id: Some(123),
                existing_archive_file_name: Some("existing_archive.zst".to_string()),
            },
        );
        content.insert(
            checksum3,
            ImportFileContent {
                file_name: "game3.rom".to_string(),
                sha1_checksum: checksum3,
                file_size: 4096,
                existing_file_info_id: None,
                existing_archive_file_name: None,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum1, checksum2, checksum3],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );

        let result = file_import_data.get_new_selected_file_names();
        assert_eq!(result.len(), 2);
        // assert included new files
        assert!(result.contains("game1.rom"));
        assert!(result.contains("game3.rom"));
        // assert excluded existing file
        assert!(!result.contains("game2.rom"));
    }

    #[test]
    fn test_get_file_import_model() {
        let checksum: Sha1Checksum = [1u8; 20];

        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
                existing_file_info_id: None,
                existing_archive_file_name: None,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );

        let model: FileImportModel = (&file_import_data).into();
        assert_eq!(model.file_type, FileType::Rom);
        assert_eq!(model.file_path.len(), 1);
        assert_eq!(model.file_path[0], PathBuf::from("/test/games.zip"));
        assert_eq!(model.new_files_file_name_filter.len(), 1);
        assert!(model.new_files_file_name_filter.contains("game.rom"));
    }
}
