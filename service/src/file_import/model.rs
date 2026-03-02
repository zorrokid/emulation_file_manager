use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use core_types::{FileSize, FileType, ImportedFile, Sha1Checksum, item_type::ItemType};
use database::{models::FileInfo, repository_manager::RepositoryManager};
use file_import::{FileImportModel, FileImportOps};

use crate::{error::Error, file_system_ops::FileSystemOps, view_models::Settings};

/*pub struct FileSetOperationDeps {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub fs_ops: Arc<dyn FileSystemOps>,
}*/

#[derive(Debug, Clone)]
pub struct FileImportMetadata {
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub is_zip_archive: bool,
}

/// Content of a file to be imported.
#[derive(Debug, Clone)]
pub struct ImportFileContent {
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,
}

/// Single file import source model including path and content info.
#[derive(Debug, Clone)]
pub struct FileImportSource {
    /// Source path to the file to be imported (e.g., zip archive)
    pub path: PathBuf,
    /// Mapping of SHA1 checksum to file content info
    pub content: HashMap<Sha1Checksum, ImportFileContent>,
}

impl FileImportSource {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            content: HashMap::new(),
        }
    }

    pub fn with_content(mut self, import_content: ImportFileContent) -> Self {
        self.content
            .insert(import_content.sha1_checksum, import_content);
        self
    }
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
    /// TODO: instead having existing info in FileImportSoure, maybe add a separate field for it?
    /// OR: maybe provide existing files as parameter for get_new_selected_file_names
    pub import_files: Vec<FileImportSource>,
}

impl FileImportData {
    pub fn new(file_type: FileType, output_dir: PathBuf) -> Self {
        Self {
            file_type,
            output_dir,
            selected_files: Vec::new(),
            import_files: Vec::new(),
        }
    }

    pub fn with_selected_file(mut self, selected_file: Sha1Checksum) -> Self {
        self.selected_files.push(selected_file);
        self
    }

    pub fn with_file_import_source(mut self, import_source: FileImportSource) -> Self {
        self.import_files.push(import_source);
        self
    }

    pub fn get_file_import_model(&self, existing_files: &[FileInfo]) -> FileImportModel {
        FileImportModel {
            file_path: self
                .import_files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<PathBuf>>(),
            output_dir: self.output_dir.clone(),
            file_type: self.file_type,
            new_files_file_name_filter: self.get_new_selected_file_names(existing_files),
        }
    }
}

impl FileImportData {
    pub fn get_new_selected_file_names(&self, existing_files: &[FileInfo]) -> HashSet<String> {
        self.import_files
            .iter()
            .flat_map(|file| {
                file.content
                    .iter()
                    .filter_map(|(sha1_checksum, import_content)| {
                        if self.selected_files.contains(sha1_checksum)
                            && !existing_files
                                .iter()
                                .any(|f| f.sha1_checksum == *sha1_checksum)
                        {
                            Some(import_content.file_name.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect::<HashSet<String>>()
    }

    pub fn is_new_files_to_be_imported(&self, existing_files: &[FileInfo]) -> bool {
        !self.get_new_selected_file_names(existing_files).is_empty()
    }
}

#[derive(Debug)]
pub struct FileImportPrepareResult {
    pub import_model: FileImportSource,
    pub import_metadata: FileImportMetadata,
}

#[derive(Debug)]
pub struct FileImportResult {
    pub file_set_id: i64,
    pub release_id: Option<i64>,
    pub imported_new_files: Vec<ImportedFile>,
    pub failed_steps: HashMap<String, Error>,
}

#[derive(Debug, Clone)]
pub struct CreateReleaseParams {
    pub release_name: String,
    pub software_title_name: String,
}

// This is used for creating a single file set from one or more import sources.
#[derive(Debug, Clone)]
pub struct FileSetImportModel {
    pub import_files: Vec<FileImportSource>,
    pub selected_files: Vec<Sha1Checksum>,
    pub system_ids: Vec<i64>,
    pub source: String, // TODO: this should be for each import source

    pub file_set_name: String,
    pub file_set_file_name: String,
    pub file_type: FileType,
    pub item_ids: Vec<i64>,
    pub item_types: Vec<ItemType>,
    /// If this is set, creates a release, links file set to it and creates a new software title and links the release to it.
    pub create_release: Option<CreateReleaseParams>,
    pub dat_file_id: Option<i64>,
}

#[derive(Debug)]
pub struct UpdateFileSetModel {
    // This contains only new import files to be added to the file set
    pub import_files: Vec<FileImportSource>,
    pub selected_files: Vec<Sha1Checksum>,
    // TODO: maybe removed files is not needed, we can determine it by comparing selected_files and
    // files already in the file set
    //pub removed_files: Vec<Sha1Checksum>,
    pub source: String, // TODO: this should be for each import source
    pub file_set_id: i64,
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub file_type: FileType,
    // TODO: remove?
    pub item_ids: Vec<i64>,
    pub item_types: Vec<ItemType>,
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
        let existing_files = vec![];
        let result = file_import_data.get_new_selected_file_names(&existing_files);
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
            },
        );
        content.insert(
            checksum2,
            ImportFileContent {
                file_name: "game2.rom".to_string(),
                sha1_checksum: checksum2,
                file_size: 2048,
            },
        );
        content.insert(
            checksum3,
            ImportFileContent {
                file_name: "game3.rom".to_string(),
                sha1_checksum: checksum3,
                file_size: 4096,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum1, checksum2, checksum3],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );

        let existing_files = vec![FileInfo {
            id: 123,
            sha1_checksum: checksum2.into(),
            file_size: 2048,
            file_type: FileType::Rom,
            archive_file_name: "archive_file_name".to_string(),
        }];

        let result = file_import_data.get_new_selected_file_names(&existing_files);
        assert_eq!(result.len(), 2);
        // assert included new files
        assert!(result.contains("game1.rom"));
        assert!(result.contains("game3.rom"));
        // assert excluded existing file
        assert!(!result.contains("game2.rom"));
    }

    #[test]
    fn test_get_file_import_model_without_existing_files() {
        let checksum: Sha1Checksum = [1u8; 20];

        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );

        let existing_files = vec![];
        let model: FileImportModel = file_import_data.get_file_import_model(&existing_files);
        assert_eq!(model.file_type, FileType::Rom);
        assert_eq!(model.file_path.len(), 1);
        assert_eq!(model.file_path[0], PathBuf::from("/test/games.zip"));
        assert_eq!(model.new_files_file_name_filter.len(), 1);
        assert!(model.new_files_file_name_filter.contains("game.rom"));
    }

    #[test]
    fn test_get_file_import_model_with_existing_files() {
        let checksum: Sha1Checksum = [1u8; 20];

        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );

        let existing_files = vec![FileInfo {
            id: 123,
            sha1_checksum: checksum.into(),
            file_size: 1024,
            file_type: FileType::Rom,
            archive_file_name: "archive_file_name".to_string(),
        }];
        let model: FileImportModel = file_import_data.get_file_import_model(&existing_files);
        assert_eq!(model.file_type, FileType::Rom);
        assert_eq!(model.file_path.len(), 1);
        assert_eq!(model.file_path[0], PathBuf::from("/test/games.zip"));
        assert!(model.new_files_file_name_filter.is_empty());
    }
}
