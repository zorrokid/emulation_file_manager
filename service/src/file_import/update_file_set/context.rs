use std::{collections::HashMap, sync::Arc};

use crate::{
    file_import::{
        common_steps::{
            check_existing_files::CheckExistingFilesContext,
            file_deletion_steps::FileDeletionStepsContext,
        },
        model::FileImportData,
    },
    file_set_deletion::model::FileDeletionResult,
};
use core_types::{FileType, ImportedFile, Sha1Checksum};
use database::{
    models::{FileInfo, FileSet},
    repository_manager::RepositoryManager,
};
use file_import::FileImportOps;

use crate::{
    file_import::common_steps::import::AddFileSetContextOps, file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct UpdateFileSetContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub file_set_id: i64,
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub source: String,

    pub file_set: Option<FileSet>,
    // files currently associated with the file set
    pub files_in_file_set: Vec<FileInfo>,

    pub file_import_data: FileImportData,
    // existing files found in the database, not yet associated with the file set
    pub existing_files: Vec<FileInfo>,
    pub new_files: Vec<FileInfo>,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
    /// To collect deletion results for files removed from the file set
    pub deletion_results: HashMap<Sha1Checksum, FileDeletionResult>,
}

impl UpdateFileSetContext {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        file_import_ops: Arc<dyn FileImportOps>,
        fs_ops: Arc<dyn FileSystemOps>,
        file_set_id: i64,
        file_import_data: FileImportData,
        file_set_name: String,
        file_set_file_name: String,
        source: String,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            file_import_ops,
            fs_ops,
            file_set_id,
            file_set: None,
            file_import_data,
            existing_files: vec![],
            imported_files: HashMap::new(),
            new_files: vec![],
            files_in_file_set: vec![],
            file_set_name,
            file_set_file_name,
            source,
            deletion_results: HashMap::new(),
        }
    }

    pub fn has_removed_files(&self) -> bool {
        // Check if there are files that were in the file set but are not in the selected files
        // anymore
        self.files_in_file_set.iter().any(|file| {
            !self
                .file_import_data
                .selected_files
                .contains(&file.sha1_checksum)
        })
    }

    pub fn get_removed_files(&self) -> Vec<FileInfo> {
        self.files_in_file_set
            .iter()
            .filter(|file| {
                !self
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
            .file_import_data
            .import_files
            .iter()
            .flat_map(|source| source.content.values())
        {
            let sha1_checksum = file.sha1_checksum;

            // file has to be selected for import
            if self
                .file_import_data
                .selected_files
                .contains(&sha1_checksum)
            {
                // file info for file is either in existing files or new files
                let file_info_id = if let Some(existing_file) = self
                    .existing_files
                    .iter()
                    .find(|f| f.sha1_checksum == sha1_checksum)
                {
                    existing_file.id
                } else if let Some(new_file) = self
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
        self.file_import_data.selected_files.clone()
    }
    fn file_type(&self) -> FileType {
        self.file_import_data.file_type
    }
    fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.repository_manager.clone()
    }
    fn set_existing_files(&mut self, existing_files: Vec<FileInfo>) {
        self.existing_files = existing_files;
    }
}

impl AddFileSetContextOps for UpdateFileSetContext {
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
            .selected_files
            .iter()
            .any(|sha1_checksum| {
                !self
                    .existing_files
                    .iter()
                    .any(|file_info| file_info.sha1_checksum == *sha1_checksum)
            })
    }
}

impl FileDeletionStepsContext for UpdateFileSetContext {
    fn repository_manager(&self) -> Arc<RepositoryManager> {
        self.repository_manager.clone()
    }

    fn file_set_id(&self) -> i64 {
        self.file_set_id
    }

    fn has_deletion_candidates(&self) -> bool {
        !self.deletion_results.is_empty()
    }

    fn deletion_results_mut(&mut self) -> &mut HashMap<Sha1Checksum, FileDeletionResult> {
        &mut self.deletion_results
    }

    fn deletion_results(&self) -> &HashMap<Sha1Checksum, FileDeletionResult> {
        &self.deletion_results
    }

    fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
        self.fs_ops.clone()
    }

    fn settings(&self) -> Arc<Settings> {
        self.settings.clone()
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use core_types::{FileType, Sha1Checksum};
    use database::{models::FileInfo, repository_manager::RepositoryManager, setup_test_db};
    use file_import::mock::MockFileImportOps;

    use crate::{
        file_import::{
            model::{FileImportData, FileImportSource, ImportFileContent},
            update_file_set::context::UpdateFileSetContext,
        },
        file_system_ops::mock::MockFileSystemOps,
    };

    async fn create_test_context(file_import_data: FileImportData) -> UpdateFileSetContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());

        UpdateFileSetContext::new(
            repository_manager,
            settings,
            Arc::new(MockFileImportOps::new()),
            Arc::new(MockFileSystemOps::new()),
            1,
            file_import_data,
            "Test File Set".to_string(),
            "test_file_set".to_string(),
            "TMMP".to_string(),
        )
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
        context.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: file_1_checksum.into(),
            file_type: FileType::Rom,
            archive_file_name: "archive_file_name_1".to_string(),
            file_size: 1024,
        });
        context.new_files.push(FileInfo {
            id: 2,
            sha1_checksum: file_2_checksum.into(),
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
        context.existing_files.push(FileInfo {
            id: 1,
            sha1_checksum: file_1_checksum.into(),
            file_type: FileType::Rom,
            archive_file_name: "archive_file_name_1".to_string(),
            file_size: 1024,
        });

        let result = context.get_file_info_ids_with_file_names();
        assert_eq!(result.len(), 0);
    }
}
