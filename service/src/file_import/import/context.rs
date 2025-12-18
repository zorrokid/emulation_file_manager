use std::{collections::HashMap, sync::Arc};

use core_types::{ImportedFile, Sha1Checksum};
use database::repository_manager::RepositoryManager;
use file_import::FileImportOps;

use crate::{
    file_import::{common_steps::import::FileImportContextOps, model::FileImportData},
    file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct FileImportContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub system_ids: Vec<i64>,
    pub source: String,
    pub file_import_data: FileImportData,
    // File set name and file name that will be created from the set of import files.
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
    pub file_set_id: Option<i64>,
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub file_system_ops: Arc<dyn FileSystemOps>,
}

impl FileImportContext {
    pub fn get_files_in_file_set(&self) -> Vec<ImportedFile> {
        let mut files_in_file_set: Vec<ImportedFile> =
            self.imported_files.values().cloned().collect();

        // add existing files that were selected
        self.file_import_data.import_files.iter().for_each(|file| {
            file.content
                .iter()
                .for_each(|(sha1_checksum, file_content)| {
                    if self.file_import_data.selected_files.contains(sha1_checksum)
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
}

impl FileImportContextOps for FileImportContext {
    fn set_imported_files(&mut self, imported_files: HashMap<Sha1Checksum, ImportedFile>) {
        self.imported_files = imported_files;
    }

    fn file_import_ops(&self) -> &Arc<dyn FileImportOps> {
        &self.file_import_ops
    }

    fn get_file_import_data(&self) -> &FileImportData {
        &self.file_import_data
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::{
        file_import::model::{FileImportSource, ImportFileContent},
        file_system_ops::mock::MockFileSystemOps,
    };
    use core_types::FileType;
    use database::setup_test_db;
    use file_import::mock::MockFileImportOps;

    fn create_file_import_data(
        selected_files: Vec<Sha1Checksum>,
        import_files: Vec<FileImportSource>,
    ) -> FileImportData {
        FileImportData {
            output_dir: PathBuf::from("/import_files"),
            file_type: FileType::Rom,
            selected_files,
            import_files,
        }
    }

    fn create_test_context(file_import_data: FileImportData) -> FileImportContext {
        let pool = async_std::task::block_on(setup_test_db());
        let repository_manager = Arc::new(RepositoryManager::new(Arc::new(pool)));
        let settings = Arc::new(Settings::default());
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let file_system_ops = Arc::new(MockFileSystemOps::new());

        FileImportContext {
            repository_manager,
            settings,
            file_import_data,
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
    fn test_get_files_in_file_set_empty() {
        let file_import_data = create_file_import_data(vec![], vec![]);
        let context = create_test_context(file_import_data);
        let result = context.get_files_in_file_set();
        assert!(result.is_empty());
    }

    #[test]
    fn test_get_files_in_file_set_includes_imported_files() {
        let checksum: Sha1Checksum = [1u8; 20];
        let file_import_data = create_file_import_data(vec![checksum], vec![]);
        let mut context = create_test_context(file_import_data);
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
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];

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

        let file_import_data = create_file_import_data(
            vec![checksum1, checksum2],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );
        let mut context = create_test_context(file_import_data);

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
        let checksum1: Sha1Checksum = [1u8; 20];
        let checksum2: Sha1Checksum = [2u8; 20];

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

        let file_import_data = create_file_import_data(
            vec![checksum1, checksum2],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );
        let mut context = create_test_context(file_import_data);
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
}
