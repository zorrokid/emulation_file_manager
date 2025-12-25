use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::file_import::{
    common_steps::{
        check_existing_files::CheckExistingFilesContext, collect_file_info::CollectFileInfoContext,
    },
    model::FileImportData,
};
use core_types::{ImportedFile, ReadFile, Sha1Checksum};
use database::{
    models::{FileInfo, FileSet},
    repository_manager::RepositoryManager,
};
use file_import::FileImportOps;

use crate::{
    file_import::common_steps::import::FileImportContextOps, file_system_ops::FileSystemOps,
    view_models::Settings,
};

pub struct AddFileToFileSetContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub file_import_ops: Arc<dyn FileImportOps>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub file_set_id: i64,

    pub file_set: Option<FileSet>,
    pub file_import_data: FileImportData,
    pub file_path: PathBuf,
    pub existing_files: Vec<FileInfo>,
    pub new_files: Vec<FileInfo>,
    pub file_info: HashMap<Sha1Checksum, ReadFile>,
    pub is_zip_archive: Option<bool>,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
}

impl AddFileToFileSetContext {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
        file_import_ops: Arc<dyn FileImportOps>,
        fs_ops: Arc<dyn FileSystemOps>,
        file_set_id: i64,
        file_import_data: FileImportData,
        file_path: &PathBuf,
    ) -> Self {
        Self {
            repository_manager,
            settings,
            file_import_ops,
            fs_ops,
            file_set_id,
            file_set: None,
            file_import_data,
            file_path: file_path.clone(),
            existing_files: vec![],
            file_info: HashMap::new(),
            is_zip_archive: None,
            imported_files: HashMap::new(),
            new_files: vec![],
        }
    }

    pub fn get_file_info_ids_with_file_names(&self) -> Vec<(i64, String)> {
        let mut file_info_ids_with_names = Vec::new();

        let import_files = self.file_import_data.import_files.iter();

        for import_file in import_files {
            for (sha1_checksum, file_content) in &import_file.content {
                // file has to be selected for import
                if self.file_import_data.selected_files.contains(sha1_checksum) {
                    // file info for file is either in existing files or new files
                    let file_info_id = if let Some(existing_file) = self
                        .existing_files
                        .iter()
                        .find(|f| f.sha1_checksum == *sha1_checksum)
                    {
                        existing_file.id
                    } else if let Some(new_file) = self
                        .new_files
                        .iter()
                        .find(|f| f.sha1_checksum == *sha1_checksum)
                    {
                        new_file.id
                    } else {
                        // this should never happen
                        panic!(
                            "File info not found for selected file with checksum: {:?}",
                            sha1_checksum
                        );
                    };

                    file_info_ids_with_names.push((file_info_id, file_content.file_name.clone()));
                }
            }
        }

        file_info_ids_with_names
    }
}

impl FileImportContextOps for AddFileToFileSetContext {
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

impl CollectFileInfoContext for AddFileToFileSetContext {
    fn is_zip_archive(&self) -> Option<bool> {
        self.is_zip_archive
    }

    fn file_import_ops(&self) -> Arc<dyn FileImportOps> {
        self.file_import_ops.clone()
    }

    fn set_file_info(&mut self, file_info: HashMap<Sha1Checksum, ReadFile>) {
        self.file_info = file_info;
    }

    fn file_path(&self) -> &PathBuf {
        &self.file_path
    }
}

impl CheckExistingFilesContext for AddFileToFileSetContext {
    fn file_info(&self) -> &HashMap<Sha1Checksum, ReadFile> {
        &self.file_info
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
    use std::{path::PathBuf, sync::Arc};

    use core_types::{FileType, Sha1Checksum};
    use database::{models::FileInfo, repository_manager::RepositoryManager, setup_test_db};
    use file_import::mock::MockFileImportOps;

    use crate::{
        file_import::{
            add_file_to_file_set::context::AddFileToFileSetContext,
            model::{FileImportData, FileImportSource, ImportFileContent},
        },
        file_system_ops::mock::MockFileSystemOps,
    };

    async fn create_test_context(file_import_data: FileImportData) -> AddFileToFileSetContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());

        AddFileToFileSetContext::new(
            repository_manager,
            settings,
            Arc::new(MockFileImportOps::new()),
            Arc::new(MockFileSystemOps::new()),
            1,
            file_import_data,
            &std::path::PathBuf::from("/path/to/file.zip"),
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
                            // TODO: in this context we don't use these fields, maybe the model should
                            // be different?
                            // Maybe existing file info was out of place here in the first place?
                            //existing_file_info_id: None,
                            //existing_archive_file_name: None,
                        },
                    ),
                    (
                        file_2_checksum,
                        ImportFileContent {
                            file_name: "file2.rom".to_string(),
                            sha1_checksum: file_2_checksum,
                            file_size: 4096,
                            // TODO: in this context we don't use these fields, maybe the model should
                            // be different?
                            // Maybe existing file info was out of place here in the first place?
                            //existing_file_info_id: None,
                            //existing_archive_file_name: None,
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
                        // TODO: in this context we don't use these fields, maybe the model should
                        // be different?
                        // Maybe existing file info was out of place here in the first place?
                        //existing_file_info_id: None,
                        //existing_archive_file_name: None,
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
