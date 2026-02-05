use std::{collections::HashMap, sync::Arc};

use core_types::{ImportedFile, Sha1Checksum, item_type::ItemType};
use database::{models::FileInfo, repository_manager::RepositoryManager};
use file_import::FileImportOps;

use crate::{
    error::Error,
    file_import::{
        common_steps::{
            check_existing_files::CheckExistingFilesContext, import::AddFileSetContextOps,
        },
        model::{CreateReleaseParams, FileImportData, ImportFileContent},
    },
    file_set_service::{CreateFileSetParams, FileSetService},
    file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct AddFileSetDeps {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

pub struct AddFileSetOps {
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub fs_ops: Arc<dyn FileSystemOps>,
}

pub struct AddFileSetInput {
    pub system_ids: Vec<i64>,
    pub file_import_data: FileImportData,
    pub create_release: Option<CreateReleaseParams>,

    // File set name and file name for file set that will be created from the set of import files.
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub source: String,
}

#[derive(Default)]
pub struct AddFileSetState {
    pub item_ids: Vec<i64>,
    pub item_types: Vec<ItemType>,

    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
    pub file_set_id: Option<i64>,
    pub release_id: Option<i64>,

    pub existing_files: Vec<FileInfo>,
    // There can be steps where failure don't abort the pipeline. Collect those failed steps during deletion, with error message
    pub failed_steps: HashMap<String, Error>,
}

pub struct AddFileSetContext {
    pub deps: AddFileSetDeps,
    pub ops: AddFileSetOps,
    pub input: AddFileSetInput,
    pub state: AddFileSetState,
}

impl AddFileSetContext {
    pub fn new(ops: AddFileSetOps, deps: AddFileSetDeps, input: AddFileSetInput) -> Self {
        Self {
            deps,
            ops,
            input,
            state: AddFileSetState::default(),
        }
    }

    pub fn get_files_in_file_set(&self) -> Vec<ImportedFile> {
        println!(
            "Getting files in file set. Imported files count: {}, Existing files count: {}",
            self.state.imported_files.len(),
            self.state.existing_files.len()
        );

        dbg!("existing files", &self.state.existing_files);

        // conbine newly imported files and existing files that were selected for import
        self.state.imported_files
            .values()
            .cloned()
            // TODO: in this case there shouldn't have been any existing files!! only newly
            // imported files - from logs (how did it manage to find existing files then??):
            // 2026-01-03T22:27:40.305488Z  INFO Executing step: check_existing_files
            // Checking for existing files in the database...
            // 2026-01-03T22:27:40.306318Z  INFO Fetched existing file info from repository existing_file_count=1
            .chain(self.state.existing_files.iter().filter_map(|file_info| {
                // TODO : this should never happen, fix the root cause
                if !self
                    .input
                    .file_import_data
                    .selected_files
                    .contains(&file_info.sha1_checksum)
                {
                    tracing::warn!(
                     existing_checksum = ?file_info.sha1_checksum,
                     archive_file_name = %file_info.archive_file_name,
                     selected_files = ?self.input.file_import_data.selected_files,

                        "File with checksum in existing files that was not selected for import!",
                    );
                    return None;
                }

                let import_files: HashMap<Sha1Checksum, ImportFileContent> = self
                    .input
                    .file_import_data
                    .import_files
                    .iter()
                    .flat_map(|import_file| import_file.content.clone())
                    .collect();

                dbg!("import files", &import_files);
                dbg!("file info sha1", &file_info.sha1_checksum);

                let original_file_name = match import_files
                    .get(&file_info.sha1_checksum)
                    // TODO: fix this // NOTE 27.1.2026: This haven't happened for a whle now.
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
                    //.map(|content| content.file_name.clone())
                    //.expect("FileInfo sha1_checksum not found in import files.");
                    {
                        Some(content) => content.file_name.clone(),
                        None => {
                            tracing::warn!(
                                existing_checksum = ?file_info.sha1_checksum,
                                archive_file_name = %file_info.archive_file_name,
                                import_files_checksums = ?import_files.keys().collect::<Vec<_>>(),
                                "Checksum in selected_files but not in import_files. Possible data inconsistency."

                            );
                            return None;
                        }
                    };

                Some(ImportedFile {
                    original_file_name,
                    sha1_checksum: file_info.sha1_checksum,
                    file_size: file_info.file_size,
                    archive_file_name: file_info.archive_file_name.clone(),
                })
            }))
            .collect()
    }

    pub fn get_file_set_service(&self) -> FileSetService {
        FileSetService::new(self.deps.repository_manager.clone())
    }

    pub fn to_create_file_set_params(&self) -> CreateFileSetParams {
        CreateFileSetParams {
            file_set_name: self.input.file_set_name.clone(),
            file_set_file_name: self.input.file_set_file_name.clone(),
            source: self.input.source.clone(),
            file_type: self.input.file_import_data.file_type,
            system_ids: self.input.system_ids.clone(),
            files_in_file_set: self.get_files_in_file_set(),
            create_release: self.input.create_release.clone(),
        }
    }
}

impl AddFileSetContextOps for AddFileSetContext {
    fn set_imported_files(&mut self, imported_files: HashMap<Sha1Checksum, ImportedFile>) {
        self.state.imported_files = imported_files;
    }

    fn file_import_ops(&self) -> &Arc<dyn FileImportOps> {
        &self.ops.file_import_ops
    }
    fn get_file_import_model(&self) -> file_import::FileImportModel {
        self.input
            .file_import_data
            .get_file_import_model(&self.state.existing_files)
    }
    fn is_new_files_to_be_imported(&self) -> bool {
        self.input
            .file_import_data
            .is_new_files_to_be_imported(&self.state.existing_files)
    }
}

impl CheckExistingFilesContext for AddFileSetContext {
    fn get_sha1_checksums(&self) -> Vec<Sha1Checksum> {
        self.input.file_import_data.selected_files.clone()
    }
    fn file_type(&self) -> core_types::FileType {
        self.input.file_import_data.file_type
    }
    fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.deps.repository_manager.clone()
    }
    fn set_existing_files(&mut self, existing_files: Vec<FileInfo>) {
        self.state.existing_files = existing_files;
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

    fn create_test_context(file_import_data: FileImportData) -> AddFileSetContext {
        let pool = async_std::task::block_on(setup_test_db());
        let repository_manager = Arc::new(RepositoryManager::new(Arc::new(pool)));
        let settings = Arc::new(Settings::default());
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let file_system_ops = Arc::new(MockFileSystemOps::new());

        let deps = AddFileSetDeps {
            repository_manager: repository_manager.clone(),
            settings: settings.clone(),
        };

        let ops = AddFileSetOps {
            file_import_ops,
            fs_ops: file_system_ops,
        };

        let input = AddFileSetInput {
            system_ids: vec![],
            file_import_data,
            create_release: None,
            file_set_name: "Test Game".to_string(),
            file_set_file_name: "test_game.zip".to_string(),
            source: "test_source".to_string(),
        };

        let state = AddFileSetState::default();

        AddFileSetContext {
            deps,
            ops,
            input,
            state,
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
        context.state.imported_files.insert(
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
        context.state.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: checksum1,
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
        context.state.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: checksum2,
            file_size: 2048,
            archive_file_name: "existing_archive_file_name".to_string(),
            file_type: FileType::Rom,
        });
        // Add a newly imported file
        context.state.imported_files.insert(
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
