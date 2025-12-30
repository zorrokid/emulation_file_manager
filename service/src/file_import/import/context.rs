use std::{collections::HashMap, sync::Arc};

use core_types::{ImportedFile, Sha1Checksum};
use database::{models::FileInfo, repository_manager::RepositoryManager};
use file_import::FileImportOps;

use crate::{
    file_import::{
        common_steps::{
            check_existing_files::CheckExistingFilesContext, import::FileImportContextOps,
        },
        model::{FileImportData, ImportFileContent},
    },
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
    pub existing_files: Vec<FileInfo>,
}

impl FileImportContext {
    pub fn get_files_in_file_set(&self) -> Vec<ImportedFile> {
        println!(
            "Getting files in file set. Imported files count: {}, Existing files count: {}",
            self.imported_files.len(),
            self.existing_files.len()
        );

        // conbine newly imported files and existing files that were selected for import
        self.imported_files
            .values()
            .cloned()
            .chain(self.existing_files.iter().map(|file_info| {
                // TODO: simplify this lookup by storing a mapping in the context?
                let import_files: HashMap<Sha1Checksum, ImportFileContent> = self
                    .file_import_data
                    .import_files
                    .iter()
                    .flat_map(|import_file| import_file.content.clone())
                    .collect();

                dbg!(&import_files);

                let sha1_checksum: Sha1Checksum = file_info
                    .sha1_checksum
                    .clone()
                    .try_into()
                    .expect("Was expecting checksum to be 20 bytes for SHA1");
                let original_file_name = import_files
                    .get(&sha1_checksum)
                    // TODO: fix this
                    // - I was importing a single file, it shouldn't be in both imported and
                    // existing files?
                    // - Another thing is that when file is in existing files, it's checksum should
                    // be in import_files as well
                    // - Yet another thing is that tokio runtime was used instead of async-std (I
                    // was expecting async-std because I explicitly use async-std in this project)
                    // - I wonder if the error occured when I added a source field entry with the
                    // following string: https://archive.org/details/trivialcrosswords1986fowlerh.
                    // Because this doesn't happen always.
                    //
                    //Getting files in file set. Imported files count: 1, Existing files count: 1
                    //
                    //thread 'tokio-runtime-worker' panicked at service/src/file_import/import/context.rs:61:22:
                    //FileInfo sha1_checksum not found in import files
                    .map(|content| content.file_name.clone())
                    .expect("FileInfo sha1_checksum not found in import files");

                ImportedFile {
                    original_file_name,
                    sha1_checksum,
                    file_size: file_info.file_size,
                    archive_file_name: file_info.archive_file_name.clone(),
                }
            }))
            .collect()
    }
}

impl FileImportContextOps for FileImportContext {
    fn set_imported_files(&mut self, imported_files: HashMap<Sha1Checksum, ImportedFile>) {
        self.imported_files = imported_files;
    }

    fn file_import_ops(&self) -> &Arc<dyn FileImportOps> {
        &self.file_import_ops
    }
    fn get_file_import_model(&self) -> file_import::FileImportModel {
        self.file_import_data
            .get_file_import_model(&self.existing_files)
    }
    fn is_new_files_to_be_imported(&self) -> bool {
        self.file_import_data
            .is_new_files_to_be_imported(&self.existing_files)
    }
}

impl CheckExistingFilesContext for FileImportContext {
    fn get_sha1_checksums(&self) -> Vec<Sha1Checksum> {
        self.file_import_data.selected_files.clone()
    }
    fn file_type(&self) -> core_types::FileType {
        self.file_import_data.file_type
    }
    fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.repository_manager.clone()
    }
    fn set_existing_files(&mut self, existing_files: Vec<FileInfo>) {
        self.existing_files = existing_files;
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
            existing_files: vec![],
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
            },
        );
        content.insert(
            checksum2,
            ImportFileContent {
                file_name: "not_selected.rom".to_string(),
                sha1_checksum: checksum2,
                file_size: 3072,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum1],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );
        let mut context = create_test_context(file_import_data);
        context.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: checksum1.to_vec(),
            file_size: 2048,
            archive_file_name: "existing_archive_file".to_string(),
            file_type: FileType::Rom,
        });

        let result = context.get_files_in_file_set();
        assert_eq!(result.len(), 1);

        let existing_file = result
            .iter()
            .find(|f| f.original_file_name == "existing_game.rom")
            .unwrap();
        assert_eq!(existing_file.archive_file_name, "existing_archive_file");
        assert_eq!(existing_file.file_size, 2048);
        assert_eq!(existing_file.sha1_checksum, checksum1);
        let not_selected_file = result
            .iter()
            .find(|f| f.original_file_name == "not_selected.rom");
        assert!(not_selected_file.is_none());
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
        context.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: checksum2.to_vec(),
            file_size: 2048,
            archive_file_name: "existing_archive_file_name".to_string(),
            file_type: FileType::Rom,
        });
        // Add a newly imported file
        context.imported_files.insert(
            checksum1,
            ImportedFile {
                original_file_name: "new_game.rom".to_string(),
                sha1_checksum: checksum1,
                file_size: 1024,
                archive_file_name: "new_archive_file_name".to_string(),
            },
        );

        let result = context.get_files_in_file_set();
        assert_eq!(result.len(), 2);

        let new_file = result
            .iter()
            .find(|f| f.original_file_name == "new_game.rom")
            .unwrap();
        assert_eq!(new_file.archive_file_name, "new_archive_file_name");

        let existing_file = result
            .iter()
            .find(|f| f.original_file_name == "existing_game.rom")
            .unwrap();
        assert_eq!(
            existing_file.archive_file_name,
            "existing_archive_file_name"
        );
    }
}
