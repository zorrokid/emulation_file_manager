use std::{path::Path, sync::Arc};

use core_types::{FileType, ImportedFile};
use database::repository_manager::RepositoryManager;
use file_import::FileImportOps;

use crate::{
    error::Error,
    file_import::{
        add_file_to_file_set::context::AddFileToFileSetContext,
        import::context::FileImportContext,
        model::{
            AddToFileSetImportModel, FileImportData, FileImportPrepareResult, FileImportResult,
            FileSetImportModel,
        },
        prepare::context::PrepareFileImportContext,
    },
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

pub struct FileImportService {
    repository_manager: Arc<RepositoryManager>,
    fs_ops: Arc<dyn FileSystemOps>,
    file_import_ops: Arc<dyn FileImportOps>,
    settings: Arc<Settings>,
}

impl std::fmt::Debug for FileImportService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileImportService").finish_non_exhaustive()
    }
}

impl FileImportService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self::new_with_ops(
            repository_manager,
            Arc::new(StdFileSystemOps),
            Arc::new(file_import::StdFileImportOps),
            settings,
        )
    }

    pub fn new_with_ops(
        repository_manager: Arc<RepositoryManager>,
        fs_ops: Arc<dyn FileSystemOps>,
        file_import_ops: Arc<dyn FileImportOps>,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            repository_manager,
            fs_ops,
            file_import_ops,
            settings,
        }
    }

    pub async fn prepare_import(
        &self,
        file_path: &Path,
        file_type: FileType,
    ) -> Result<FileImportPrepareResult, Error> {
        let mut context = PrepareFileImportContext::new(
            self.repository_manager.clone(),
            file_path,
            file_type,
            self.fs_ops.clone(),
            self.file_import_ops.clone(),
        );
        let pipeline = Pipeline::<PrepareFileImportContext>::new();
        match pipeline.execute(&mut context).await {
            Ok(_) => {
                let import_model = context.get_imported_file_info();
                let import_metadata = context.import_metadata.ok_or_else(|| {
                    Error::FileImportError("Import metadata not set after preparation".to_string())
                })?;
                Ok(FileImportPrepareResult {
                    import_model,
                    import_metadata,
                })
            }
            Err(err) => {
                tracing::error!(error = %err, "Failed to prepare file import");
                Err(err)
            }
        }
    }

    pub async fn import(
        &self,
        import_model: FileSetImportModel,
    ) -> Result<FileImportResult, Error> {
        let file_type = import_model.file_type;
        let output_dir = self.settings.collection_root_dir.clone();
        let file_import_data = FileImportData {
            output_dir,
            file_type,
            selected_files: import_model.selected_files,
            import_files: import_model.import_files,
        };

        let mut context = FileImportContext {
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
            file_import_ops: self.file_import_ops.clone(),
            file_system_ops: self.fs_ops.clone(),

            file_import_data,
            system_ids: import_model.system_ids,
            source: import_model.source,
            file_set_name: import_model.file_set_name,
            file_set_file_name: import_model.file_set_file_name,

            imported_files: std::collections::HashMap::new(),
            file_set_id: None,
            existing_files: vec![],
        };
        let pipeline = Pipeline::<FileImportContext>::new();
        let result = pipeline.execute(&mut context).await;
        match (result, context.file_set_id) {
            (Ok(_), Some(id)) => Ok(FileImportResult {
                file_set_id: id,
                imported_new_files: context
                    .imported_files
                    .values()
                    .cloned()
                    .collect::<Vec<ImportedFile>>(),
            }),
            (Err(err), _) => Err(err),
            (_, None) => Err(Error::FileImportError(
                "File set ID not set after import".to_string(),
            )),
        }
    }

    pub async fn import_and_add_files_to_file_set(
        &self,
        import_model: AddToFileSetImportModel,
    ) -> Result<FileImportResult, Error> {
        let file_import_data = FileImportData {
            output_dir: self.settings.collection_root_dir.clone(),
            file_type: import_model.file_type, // TODO make this optional? this is required only
            // when adding new file set
            selected_files: import_model.selected_files,
            import_files: import_model.import_files,
        };
        let mut context = AddFileToFileSetContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            self.file_import_ops.clone(),
            self.fs_ops.clone(),
            import_model.file_set_id,
            file_import_data,
        );
        let pipeline = Pipeline::<AddFileToFileSetContext>::new();
        let res = pipeline.execute(&mut context).await;
        match res {
            Ok(_) => Ok(FileImportResult {
                file_set_id: import_model.file_set_id,
                imported_new_files: context
                    .imported_files
                    .values()
                    .cloned()
                    .collect::<Vec<ImportedFile>>(),
            }),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use core_types::ImportedFile;
    use database::setup_test_db;
    use file_import::mock::MockFileImportOps;

    use crate::{
        file_import::model::{FileImportSource, ImportFileContent},
        file_system_ops::mock::MockFileSystemOps,
    };

    use super::*;

    #[async_std::test]
    async fn test_import() {
        let test_db = setup_test_db().await;
        let repository_manager = Arc::new(RepositoryManager::new(Arc::new(test_db)));

        let system_id = repository_manager
            .get_system_repository()
            .add_system("Atari ST")
            .await
            .unwrap();

        let settings = Arc::new(Settings {
            collection_root_dir: std::path::PathBuf::from("/tmp/collection"),
            ..Default::default()
        });

        let sha1_checksum = [0u8; 20];
        let file_name = "test_game.st".to_string();
        let file_size = 2048;
        let path_str = "/path/to/test_game.zip";
        let file_set_name = "Test Game".to_string();

        let file_import_ops = Arc::new(MockFileImportOps::new());
        file_import_ops.add_imported_file(
            sha1_checksum,
            ImportedFile {
                original_file_name: file_name.clone(),
                sha1_checksum,
                file_size,
                archive_file_name: "archive_file_name".to_string(),
            },
        );

        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(path_str);

        let service = FileImportService {
            repository_manager: repository_manager.clone(),
            fs_ops,
            file_import_ops,
            settings,
        };

        let file_import_source =
            FileImportSource::new(path_str.into()).with_content(ImportFileContent {
                file_name,
                sha1_checksum,
                file_size,
            });

        let file_set_import_model = FileSetImportModel {
            file_type: FileType::Rom,
            selected_files: vec![sha1_checksum],
            import_files: vec![file_import_source],
            system_ids: vec![system_id],
            source: "test_source".to_string(),
            file_set_name: file_set_name.clone(),
            file_set_file_name: "test_game.zip".to_string(),
        };

        let result = service.import(file_set_import_model).await;
        assert!(result.is_ok());

        let result = result.unwrap();

        assert_eq!(result.imported_new_files.len(), 1);
        assert_eq!(result.imported_new_files[0].sha1_checksum, sha1_checksum);

        let file_set = repository_manager
            .get_file_set_repository()
            .get_file_set(result.file_set_id)
            .await
            .unwrap();

        assert_eq!(file_set.name, file_set_name);

        let file_set_files = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set.id)
            .await
            .unwrap();

        assert_eq!(file_set_files.len(), 1);
        assert_eq!(file_set_files[0].sha1_checksum, sha1_checksum);
    }

    #[async_std::test]
    async fn test_add_file_to_file_set() {
        let test_db = setup_test_db().await;
        let repository_manager = Arc::new(RepositoryManager::new(Arc::new(test_db)));

        let system_id = repository_manager
            .get_system_repository()
            .add_system("Atari ST")
            .await
            .unwrap();

        let settings = Arc::new(Settings {
            collection_root_dir: std::path::PathBuf::from("/tmp/collection"),
            ..Default::default()
        });

        // First, create an existing file set with one file, to which we will add another file
        let existing_file_checksum = [1u8; 20];
        let existing_file_name = "existing_game.st".to_string();
        let existing_file_size = 2048;

        let file_set_id = repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Existing File Set",
                "Existing File Set",
                &FileType::DiskImage,
                "Source",
                &[ImportedFile {
                    original_file_name: existing_file_name,
                    sha1_checksum: existing_file_checksum,
                    file_size: existing_file_size,
                    archive_file_name: "archive_file_name".to_string(),
                }],
                &[system_id],
            )
            .await
            .unwrap();

        // Now, prepare to add a new file to the existing file set
        let new_file_sha1_checksum = [0u8; 20];
        let new_file_name = "additional_game.st".to_string();
        let new_file_size = 4096;
        let new_file_path_str = "/path/to/additional_game.zip";

        let file_import_ops = Arc::new(MockFileImportOps::new());
        file_import_ops.add_imported_file(
            new_file_sha1_checksum,
            ImportedFile {
                original_file_name: new_file_name.clone(),
                sha1_checksum: new_file_sha1_checksum,
                file_size: new_file_size,
                archive_file_name: "archive_file_name".to_string(),
            },
        );

        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(new_file_path_str);

        let service = FileImportService {
            repository_manager: repository_manager.clone(),
            fs_ops,
            file_import_ops,
            settings,
        };

        let file_import_source =
            FileImportSource::new(new_file_path_str.into()).with_content(ImportFileContent {
                file_name: new_file_name,
                sha1_checksum: new_file_sha1_checksum,
                file_size: new_file_size,
            });

        let add_to_file_set_import_model = AddToFileSetImportModel {
            file_set_id,
            selected_files: vec![new_file_sha1_checksum],
            import_files: vec![file_import_source],
            file_type: FileType::DiskImage, // TODO this shouldn't be required
            source: "test_source".to_string(), // TODO: source should be file specific, not file
                                            // sest specific
        };

        // Perform the addition
        let result = service
            .import_and_add_files_to_file_set(add_to_file_set_import_model)
            .await;
        assert!(result.is_ok());

        let result = result.unwrap();

        assert_eq!(result.imported_new_files.len(), 1);
        assert_eq!(
            result.imported_new_files[0].sha1_checksum,
            new_file_sha1_checksum
        );

        let file_set_files = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        assert_eq!(file_set_files.len(), 2);
        let new_file_found = file_set_files
            .iter()
            .any(|file_info| file_info.sha1_checksum == new_file_sha1_checksum);
        assert!(new_file_found);
    }
}
