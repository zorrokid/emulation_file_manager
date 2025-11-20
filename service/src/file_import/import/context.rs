use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use core_types::{FileType, ImportedFile, Sha1Checksum};
use database::repository_manager::RepositoryManager;
use file_import::{FileImportModel, FileImportOps};

use crate::{
    file_import::model::FileImportModel as ServiceFileImportModel, file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct FileImportContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub selected_files: Vec<Sha1Checksum>,
    pub file_type: FileType,
    // It is possible to create a file set from multiple sets of import files.
    // From this collection of files, the ones that are in selected_files and are new files, will be imported.
    // If the files are existing files, they will be just linked to the file set.
    // When files are imported, their archive file names will be populated here.
    pub import_files: Vec<ServiceFileImportModel>,
    pub system_ids: Vec<i64>,
    pub source: String,
    // File set name and file name that will be created from the set of import files.
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
    pub file_set_id: Option<i64>,
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub file_system_ops: Arc<dyn FileSystemOps>,
}

impl FileImportContext {
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

    pub fn get_files_in_file_set(&self) -> Vec<ImportedFile> {
        let mut files_in_file_set: Vec<ImportedFile> =
            self.imported_files.values().cloned().collect();

        // add existing files that were selected
        self.import_files.iter().for_each(|file| {
            file.content
                .iter()
                .for_each(|(sha1_checksum, file_content)| {
                    if self.selected_files.contains(sha1_checksum)
                        && let Some(existing_archive_file_name) =
                            &file_content.existing_archive_file_name
                    {
                        files_in_file_set.push(ImportedFile {
                            original_file_name: file_content.file_name.clone(),
                            sha1_checksum: *sha1_checksum,
                            file_size: file_content.file_size,
                            archive_file_name: existing_archive_file_name.clone(),
                        });
                    }
                });
        });

        files_in_file_set
    }

    pub fn get_file_import_model(&self) -> FileImportModel {
        let target_path = self.settings.get_file_type_path(&self.file_type);
        FileImportModel {
            file_path: self
                .import_files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<PathBuf>>(),
            output_dir: target_path.to_path_buf(),
            file_name: self.file_set_file_name.clone(),
            file_set_name: self.file_set_name.clone(),
            file_type: self.file_type,
            new_files_file_name_filter: self.get_new_selected_file_names(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{file_import::model::ImportFileContent, file_system_ops::mock::MockFileSystemOps};
    use database::setup_test_db;
    use file_import::mock::MockFileImportOps;

    fn create_test_context() -> FileImportContext {
        let pool = async_std::task::block_on(setup_test_db());
        let repository_manager = Arc::new(RepositoryManager::new(Arc::new(pool)));
        let settings = Arc::new(Settings::default());
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let file_system_ops = Arc::new(MockFileSystemOps::new());

        FileImportContext {
            repository_manager,
            settings,
            selected_files: vec![],
            file_type: FileType::Rom,
            import_files: vec![],
            system_ids: vec![],
            source: "test_source".to_string(),
            file_set_name: "Test Game".to_string(),
            file_set_file_name: "test_game.zip".to_string(),
            imported_files: HashMap::new(),
            file_set_id: None,
            file_import_ops,
            file_system_ops,
        }
    }

    #[test]
    fn test_get_new_selected_file_names_empty() {
        let context = create_test_context();
        let result = context.get_new_selected_file_names();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_new_selected_file_names_filters_new_files() {
        let mut context = create_test_context();
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];
        let checksum3: Sha1Checksum = [3u8; 20];

        context.selected_files = vec![checksum1, checksum2, checksum3];

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

        context.import_files = vec![ServiceFileImportModel {
            path: PathBuf::from("/test/games.zip"),
            content,
        }];

        let result = context.get_new_selected_file_names();
        assert_eq!(result.len(), 2);
        assert!(result.contains("game1.rom"));
        assert!(result.contains("game3.rom"));
        assert!(!result.contains("game2.rom"));
    }

    #[test]
    fn test_get_files_in_file_set_empty() {
        let context = create_test_context();
        let result = context.get_files_in_file_set();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_files_in_file_set_includes_imported_files() {
        let mut context = create_test_context();
        let checksum: Sha1Checksum = [1u8; 20];

        context.imported_files.insert(
            checksum,
            ImportedFile {
                original_file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
                archive_file_name: "archive123.zst".to_string(),
            },
        );

        let result = context.get_files_in_file_set();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].original_file_name, "game.rom");
        assert_eq!(result[0].archive_file_name, "archive123.zst");
    }

    #[test]
    fn test_get_files_in_file_set_includes_existing_selected_files() {
        let mut context = create_test_context();
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];

        context.selected_files = vec![checksum1, checksum2];

        let mut content = HashMap::new();
        content.insert(
            checksum1,
            ImportFileContent {
                file_name: "existing_game.rom".to_string(),
                sha1_checksum: checksum1,
                file_size: 2048,
                existing_file_info_id: Some(456),
                existing_archive_file_name: Some("existing_archive.zst".to_string()),
            },
        );
        content.insert(
            checksum2,
            ImportFileContent {
                file_name: "not_selected.rom".to_string(),
                sha1_checksum: checksum2,
                file_size: 3072,
                existing_file_info_id: Some(789),
                existing_archive_file_name: Some("not_selected_archive.zst".to_string()),
            },
        );

        context.import_files = vec![ServiceFileImportModel {
            path: PathBuf::from("/test/games.zip"),
            content,
        }];

        let result = context.get_files_in_file_set();
        assert_eq!(result.len(), 2);

        let existing_file = result
            .iter()
            .find(|f| f.original_file_name == "existing_game.rom")
            .unwrap();
        assert_eq!(existing_file.archive_file_name, "existing_archive.zst");
        assert_eq!(existing_file.file_size, 2048);
    }

    #[test]
    fn test_get_files_in_file_set_combines_imported_and_existing() {
        let mut context = create_test_context();
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];

        context.selected_files = vec![checksum1, checksum2];

        // Add a newly imported file
        context.imported_files.insert(
            checksum1,
            ImportedFile {
                original_file_name: "new_game.rom".to_string(),
                sha1_checksum: checksum1,
                file_size: 1024,
                archive_file_name: "new_archive123.zst".to_string(),
            },
        );

        // Add an existing file
        let mut content = HashMap::new();
        content.insert(
            checksum2,
            ImportFileContent {
                file_name: "existing_game.rom".to_string(),
                sha1_checksum: checksum2,
                file_size: 2048,
                existing_file_info_id: Some(456),
                existing_archive_file_name: Some("existing_archive.zst".to_string()),
            },
        );

        context.import_files = vec![ServiceFileImportModel {
            path: PathBuf::from("/test/games.zip"),
            content,
        }];

        let result = context.get_files_in_file_set();
        assert_eq!(result.len(), 2);

        let new_file = result
            .iter()
            .find(|f| f.original_file_name == "new_game.rom")
            .unwrap();
        assert_eq!(new_file.archive_file_name, "new_archive123.zst");

        let existing_file = result
            .iter()
            .find(|f| f.original_file_name == "existing_game.rom")
            .unwrap();
        assert_eq!(existing_file.archive_file_name, "existing_archive.zst");
    }

    #[test]
    fn test_get_file_import_model() {
        let mut context = create_test_context();
        let checksum: Sha1Checksum = [1u8; 20];

        context.selected_files = vec![checksum];
        context.file_set_name = "Test Game".to_string();
        context.file_set_file_name = "test_game.zip".to_string();
        context.file_type = FileType::Rom;

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

        context.import_files = vec![ServiceFileImportModel {
            path: PathBuf::from("/test/games.zip"),
            content,
        }];

        let model = context.get_file_import_model();
        assert_eq!(model.file_name, "test_game.zip");
        assert_eq!(model.file_set_name, "Test Game");
        assert_eq!(model.file_type, FileType::Rom);
        assert_eq!(model.file_path.len(), 1);
        assert_eq!(model.file_path[0], PathBuf::from("/test/games.zip"));
        assert_eq!(model.new_files_file_name_filter.len(), 1);
        assert!(model.new_files_file_name_filter.contains("game.rom"));
    }
}
