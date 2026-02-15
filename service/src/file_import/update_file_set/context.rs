use std::{collections::HashMap, sync::Arc};

use crate::{
    error::Error,
    file_import::{
        common_steps::{
            check_existing_files::CheckExistingFilesContext,
            file_deletion_steps::FileDeletionStepsContext,
        },
        model::FileImportData,
    },
    file_set_deletion::model::FileDeletionResult,
};
use core_types::{FileType, ImportedFile, Sha1Checksum, item_type::ItemType};
use database::{
    models::{FileInfo, FileSet},
    repository_manager::RepositoryManager,
};
use file_import::FileImportOps;

use crate::{
    file_import::common_steps::import::AddFileSetContextOps, file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct UpdateFileSetDeps {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

pub struct UpdateFileSetOps {
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub fs_ops: Arc<dyn FileSystemOps>,
}

pub struct UpdateFileSetInput {
    pub file_set_id: i64,
    pub file_import_data: FileImportData,
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub source: String,
    // TODO: rmove?
    pub item_ids: Vec<i64>,
    pub item_types: Vec<ItemType>,
}

#[derive(Default)]
pub struct UpdateFileSetState {
    pub file_set: Option<FileSet>,
    /// files currently associated with the file set
    pub files_in_file_set: Vec<FileInfo>,
    /// existing files found in the database, not yet associated with the file set
    pub existing_files: Vec<FileInfo>,
    pub new_files: Vec<FileInfo>,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
    /// To collect deletion results for files removed from the file set
    pub deletion_results: HashMap<Sha1Checksum, FileDeletionResult>,
    /// There can be steps where failure don't abort the pipeline. Collect those failed steps during deletion, with error message
    pub failed_steps: HashMap<String, Error>,
}

pub struct UpdateFileSetContext {
    pub deps: UpdateFileSetDeps,
    pub ops: UpdateFileSetOps,
    pub input: UpdateFileSetInput,
    pub state: UpdateFileSetState,
}

impl UpdateFileSetContext {
    pub fn new(deps: UpdateFileSetDeps, ops: UpdateFileSetOps, input: UpdateFileSetInput) -> Self {
        Self {
            deps,
            ops,
            input,
            state: UpdateFileSetState::default(),
        }
    }

    pub fn has_removed_files(&self) -> bool {
        // Check if there are files that were in the file set but are not in the selected files
        // anymore
        self.state.files_in_file_set.iter().any(|file| {
            !self
                .input
                .file_import_data
                .selected_files
                .contains(&file.sha1_checksum)
        })
    }

    pub fn get_removed_files(&self) -> Vec<FileInfo> {
        self.state
            .files_in_file_set
            .iter()
            .filter(|file| {
                !self
                    .input
                    .file_import_data
                    .selected_files
                    .contains(&file.sha1_checksum)
            })
            .cloned()
            .collect()
    }

    pub fn get_file_info_ids_with_file_names(&self) -> Vec<(i64, String)> {
        let mut file_info_ids_with_names = Vec::new();

        for file in self
            .input
            .file_import_data
            .import_files
            .iter()
            .flat_map(|source| source.content.values())
        {
            let sha1_checksum = file.sha1_checksum;

            // file has to be selected for import
            if self
                .input
                .file_import_data
                .selected_files
                .contains(&sha1_checksum)
            {
                // file info for file is either in existing files or new files
                let file_info_id = if let Some(existing_file) = self
                    .state
                    .existing_files
                    .iter()
                    .find(|f| f.sha1_checksum == sha1_checksum)
                {
                    existing_file.id
                } else if let Some(new_file) = self
                    .state
                    .new_files
                    .iter()
                    .find(|f| f.sha1_checksum == sha1_checksum)
                {
                    new_file.id
                } else {
                    // this should never happen
                    panic!(
                        "File info not found for selected file with checksum: {:?}",
                        sha1_checksum
                    );
                };

                file_info_ids_with_names.push((file_info_id, file.file_name.clone()));
            }
        }

        file_info_ids_with_names
    }
}

impl CheckExistingFilesContext for UpdateFileSetContext {
    fn get_sha1_checksums(&self) -> Vec<Sha1Checksum> {
        self.input.file_import_data.selected_files.clone()
    }
    fn file_type(&self) -> FileType {
        self.input.file_import_data.file_type
    }
    fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.deps.repository_manager.clone()
    }
    fn set_existing_files(&mut self, existing_files: Vec<FileInfo>) {
        self.state.existing_files = existing_files;
    }
}

impl AddFileSetContextOps for UpdateFileSetContext {
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
            .selected_files
            .iter()
            .any(|sha1_checksum| {
                !self
                    .state
                    .existing_files
                    .iter()
                    .any(|file_info| file_info.sha1_checksum == *sha1_checksum)
            })
    }
}

impl FileDeletionStepsContext for UpdateFileSetContext {
    fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.deps.repository_manager.clone()
    }

    fn file_set_id(&self) -> i64 {
        self.input.file_set_id
    }

    fn has_deletion_candidates(&self) -> bool {
        !self.state.deletion_results.is_empty()
    }

    fn deletion_results_mut(&mut self) -> &mut HashMap<Sha1Checksum, FileDeletionResult> {
        &mut self.state.deletion_results
    }

    fn deletion_results(&self) -> &HashMap<Sha1Checksum, FileDeletionResult> {
        &self.state.deletion_results
    }

    fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
        self.ops.fs_ops.clone()
    }

    fn settings(&self) -> Arc<Settings> {
        self.deps.settings.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc};

    use core_types::{FileType, Sha1Checksum};
    use database::{models::FileInfo, repository_manager::RepositoryManager, setup_test_db};
    use file_import::mock::MockFileImportOps;

    use crate::{
        file_import::{
            model::{FileImportData, FileImportSource, ImportFileContent},
            update_file_set::context::{UpdateFileSetContext, UpdateFileSetInput},
        },
        file_system_ops::mock::MockFileSystemOps,
    };

    async fn create_test_context(file_import_data: FileImportData) -> UpdateFileSetContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());

        let deps = UpdateFileSetDeps {
            repository_manager: repository_manager.clone(),
            settings: settings.clone(),
        };

        let ops = UpdateFileSetOps {
            file_import_ops: Arc::new(MockFileImportOps::new()),
            fs_ops: Arc::new(MockFileSystemOps::new()),
        };

        let input = UpdateFileSetInput {
            file_set_id: 0,
            file_import_data,
            file_set_name: "Test File Set".to_string(),
            file_set_file_name: "test_file_set".to_string(),
            source: "TMMP".to_string(),
            item_ids: vec![],
            item_types: vec![],
        };

        UpdateFileSetContext::new(deps, ops, input)
    }

    #[async_std::test]
    async fn test_get_file_info_ids_with_file_names() {
        let file_1_checksum: Sha1Checksum = [1u8; 20];
        let file_2_checksum: Sha1Checksum = [2u8; 20];
        let file_import_data = FileImportData {
            file_type: core_types::FileType::Rom,
            selected_files: vec![file_1_checksum, file_2_checksum],
            output_dir: std::path::PathBuf::from("/imported/files"),
            import_files: vec![FileImportSource {
                path: PathBuf::from("/tmmp/source1"),
                content: vec![
                    (
                        file_1_checksum,
                        ImportFileContent {
                            file_name: "file1.rom".to_string(),
                            sha1_checksum: file_1_checksum,
                            file_size: 2048,
                        },
                    ),
                    (
                        file_2_checksum,
                        ImportFileContent {
                            file_name: "file2.rom".to_string(),
                            sha1_checksum: file_2_checksum,
                            file_size: 4096,
                        },
                    ),
                ]
                .into_iter()
                .collect(),
            }],
        };
        let mut context = create_test_context(file_import_data).await;
        context.state.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: file_1_checksum,
            file_type: FileType::Rom,
            archive_file_name: "archive_file_name_1".to_string(),
            file_size: 1024,
        });
        context.state.new_files.push(FileInfo {
            id: 2,
            sha1_checksum: file_2_checksum,
            file_type: FileType::Rom,
            archive_file_name: "archive_file_name_2".to_string(),
            file_size: 4096,
        });

        let result = context.get_file_info_ids_with_file_names();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&(1, "file1.rom".to_string())));
        assert!(result.contains(&(2, "file2.rom".to_string())));
    }

    #[async_std::test]
    async fn test_get_file_info_ids_with_file_names_wihout_selected_files() {
        let file_1_checksum: Sha1Checksum = [1u8; 20];
        let file_import_data = FileImportData {
            file_type: core_types::FileType::Rom,
            selected_files: vec![],
            output_dir: std::path::PathBuf::from("/imported/files"),
            import_files: vec![FileImportSource {
                path: PathBuf::from("/tmmp/source1"),
                content: vec![(
                    file_1_checksum,
                    ImportFileContent {
                        file_name: "file1.rom".to_string(),
                        sha1_checksum: file_1_checksum,
                        file_size: 2048,
                    },
                )]
                .into_iter()
                .collect(),
            }],
        };
        let mut context = create_test_context(file_import_data).await;
        context.state.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: file_1_checksum,
            file_type: FileType::Rom,
            archive_file_name: "archive_file_name_1".to_string(),
            file_size: 1024,
        });

        let result = context.get_file_info_ids_with_file_names();
        assert_eq!(result.len(), 0);
    }
}
