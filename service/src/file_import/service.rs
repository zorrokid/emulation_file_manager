use std::{path::Path, sync::Arc};

use core_types::{FileType, ImportedFile};
use database::repository_manager::RepositoryManager;
use file_import::FileImportOps;
use file_metadata::file_metadata_ops::{FileMetadataOps, StdFileMetadataOps};

use crate::{
    error::Error,
    file_import::{
        add_file_set::context::{
            AddFileSetContext, AddFileSetDeps, AddFileSetInput, AddFileSetOps,
        },
        model::{
            FileImportData, FileImportPrepareResult, FileImportResult, FileSetImportModel,
            UpdateFileSetModel,
        },
        prepare::context::PrepareFileImportContext,
        update_file_set::context::{
            UpdateFileSetContext, UpdateFileSetDeps, UpdateFileSetInput, UpdateFileSetOps,
        },
    },
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

pub struct FileImportService {
    repository_manager: Arc<RepositoryManager>,
    fs_ops: Arc<dyn FileSystemOps>,
    file_import_ops: Arc<dyn FileImportOps>,
    file_metadata_ops: Arc<dyn FileMetadataOps>,
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
            Arc::new(StdFileMetadataOps),
            settings,
        )
    }

    pub fn new_with_ops(
        repository_manager: Arc<RepositoryManager>,
        fs_ops: Arc<dyn FileSystemOps>,
        file_import_ops: Arc<dyn FileImportOps>,
        file_metadata_ops: Arc<dyn FileMetadataOps>,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            repository_manager,
            fs_ops,
            file_import_ops,
            file_metadata_ops,
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
            self.file_metadata_ops.clone(),
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

    fn get_output_dir_for_file_type(&self, file_type: &FileType) -> std::path::PathBuf {
        self.settings.get_file_type_path(file_type)
    }

    pub async fn create_file_set(
        &self,
        import_model: FileSetImportModel,
    ) -> Result<FileImportResult, Error> {
        let file_type = import_model.file_type;
        let output_dir = self.get_output_dir_for_file_type(&file_type);
        let file_import_data = FileImportData {
            output_dir,
            file_type,
            selected_files: import_model.selected_files,
            import_files: import_model.import_files,
        };

        let ops = AddFileSetOps {
            file_import_ops: self.file_import_ops.clone(),
            fs_ops: self.fs_ops.clone(),
        };

        let deps = AddFileSetDeps {
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
        };

        let input = AddFileSetInput {
            file_import_data,
            file_set_name: import_model.file_set_name,
            file_set_file_name: import_model.file_set_file_name,
            source: import_model.source,
            system_ids: import_model.system_ids,
            create_release: import_model.create_release,
        };

        let mut context = AddFileSetContext::new(ops, deps, input);

        let pipeline = Pipeline::<AddFileSetContext>::new();
        let result = pipeline.execute(&mut context).await;
        match (result, context.state.file_set_id) {
            (Ok(_), Some(id)) => Ok(FileImportResult {
                file_set_id: id,
                release_id: context.state.release_id,
                imported_new_files: context
                    .state
                    .imported_files
                    .values()
                    .cloned()
                    .collect::<Vec<ImportedFile>>(),
                failed_steps: context.state.failed_steps,
            }),
            (Err(err), _) => Err(err),
            (_, None) => Err(Error::FileImportError(
                "File set ID not set after import".to_string(),
            )),
        }
    }

    pub async fn update_file_set(
        &self,
        import_model: UpdateFileSetModel,
    ) -> Result<FileImportResult, Error> {
        let file_import_data = FileImportData {
            output_dir: self.get_output_dir_for_file_type(&import_model.file_type),
            file_type: import_model.file_type,
            selected_files: import_model.selected_files,
            import_files: import_model.import_files,
        };
        let deps = UpdateFileSetDeps {
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
        };
        let ops = UpdateFileSetOps {
            file_import_ops: self.file_import_ops.clone(),
            fs_ops: self.fs_ops.clone(),
        };
        let input = UpdateFileSetInput {
            file_set_id: import_model.file_set_id,
            file_set_name: import_model.file_set_name,
            file_set_file_name: import_model.file_set_file_name,
            source: import_model.source,
            item_ids: import_model.item_ids,
            file_import_data,
            item_types: import_model.item_types,
        };
        let mut context = UpdateFileSetContext::new(deps, ops, input);
        let pipeline = Pipeline::<UpdateFileSetContext>::new();
        let res = pipeline.execute(&mut context).await;
        match res {
            Ok(_) => Ok(FileImportResult {
                file_set_id: import_model.file_set_id,
                release_id: None,
                imported_new_files: context
                    .state
                    .imported_files
                    .values()
                    .cloned()
                    .collect::<Vec<ImportedFile>>(),
                failed_steps: context.state.failed_steps,
            }),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use core_types::{FileSyncStatus, ImportedFile, ReadFile};
    use database::setup_test_db;
    use file_import::mock::MockFileImportOps;
    use file_metadata::file_metadata_ops::mock::MockFileMetadataOps;

    use crate::{
        file_import::model::{FileImportSource, ImportFileContent},
        file_system_ops::mock::MockFileSystemOps,
    };

    use super::*;

    async fn create_repository_manager() -> Arc<RepositoryManager> {
        let test_db = setup_test_db().await;
        Arc::new(RepositoryManager::new(Arc::new(test_db)))
    }

    fn create_settings() -> Arc<Settings> {
        Arc::new(Settings {
            collection_root_dir: std::path::PathBuf::from("/tmp/collection"),
            s3_sync_enabled: true,
            ..Default::default()
        })
    }

    async fn create_system(repository_manager: &Arc<RepositoryManager>) -> i64 {
        repository_manager
            .get_system_repository()
            .add_system("system_name")
            .await
            .unwrap()
    }

    #[async_std::test]
    async fn test_import() {
        let repository_manager = create_repository_manager().await;
        let system_id = create_system(&repository_manager).await;
        let settings = create_settings();

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

        let file_metadata_ops = Arc::new(MockFileMetadataOps::new());

        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(path_str);

        let service = FileImportService {
            repository_manager: repository_manager.clone(),
            fs_ops,
            file_import_ops,
            file_metadata_ops,
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
            item_ids: vec![],
            item_types: vec![],
            create_release: None,
        };

        let result = service.create_file_set(file_set_import_model).await;
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
    async fn test_update_file_set_add_new_file() {
        let repository_manager = create_repository_manager().await;
        let system_id = create_system(&repository_manager).await;
        let settings = create_settings();

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

        let file_metadata_ops = Arc::new(MockFileMetadataOps::new());

        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(new_file_path_str);

        let service = FileImportService {
            repository_manager: repository_manager.clone(),
            fs_ops,
            file_import_ops,
            file_metadata_ops,
            settings,
        };

        let file_import_source =
            FileImportSource::new(new_file_path_str.into()).with_content(ImportFileContent {
                file_name: new_file_name,
                sha1_checksum: new_file_sha1_checksum,
                file_size: new_file_size,
            });

        let add_to_file_set_import_model = UpdateFileSetModel {
            file_set_id,
            // Also the existing file has to be selected, otherwise it would be removed from the
            // set
            selected_files: vec![new_file_sha1_checksum, existing_file_checksum],
            import_files: vec![file_import_source],
            file_type: FileType::DiskImage,
            // TODO: source should be file specific, not file set specific
            source: "test_source".to_string(),
            file_set_name: "".to_string(),
            file_set_file_name: "".to_string(),
            item_ids: vec![],
            item_types: vec![],
        };

        // Perform the addition
        let result = service.update_file_set(add_to_file_set_import_model).await;
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
        let new_file = &file_set_files
            .iter()
            .find(|file_info| file_info.sha1_checksum == new_file_sha1_checksum);
        assert!(new_file.is_some());
        let new_file_info_id = new_file.unwrap().id;

        // assert that file is marked for cloud sync (enabled in settings)
        let sync_logs = repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(new_file_info_id)
            .await
            .unwrap();

        assert!(!sync_logs.is_empty());
        assert_eq!(sync_logs[0].status, FileSyncStatus::UploadPending);
    }

    #[async_std::test]
    async fn test_update_file_set_unlink_file_from_file_set_not_linked_to_other_file_set() {
        let repository_manager = create_repository_manager().await;
        let system_id = create_system(&repository_manager).await;
        let settings = create_settings();

        // First, create an existing file set with one file, which we will remove
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

        let file_info = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(&[existing_file_checksum], FileType::DiskImage)
            .await
            .unwrap()
            .pop()
            .unwrap();

        // simulate a successful cloud sync for the file in file set
        repository_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info.id,
                FileSyncStatus::UploadCompleted,
                "",
                file_info.generate_cloud_key().as_str(),
            )
            .await
            .unwrap();

        let file_import_ops = Arc::new(MockFileImportOps::new());
        let file_metadata_ops = Arc::new(MockFileMetadataOps::new());

        let fs_ops = Arc::new(MockFileSystemOps::new());

        let service = FileImportService {
            repository_manager: repository_manager.clone(),
            fs_ops,
            file_import_ops,
            file_metadata_ops,
            settings,
        };

        let update_file_set_model = UpdateFileSetModel {
            file_set_id,
            // no files are selected, so the existing file will be unlinked from the set
            selected_files: vec![],
            import_files: vec![],
            file_type: FileType::DiskImage,
            // TODO: source should be file specific, not file set specific
            source: "test_source".to_string(),
            file_set_name: "".to_string(),
            file_set_file_name: "".to_string(),
            item_ids: vec![],
            item_types: vec![],
        };

        // Perform the addition
        let result = service.update_file_set(update_file_set_model).await;
        assert!(result.is_ok());

        let file_set_files = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        // Because file was linked only to this file set, it should be removed completely
        let file_infos = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(&[existing_file_checksum], FileType::DiskImage)
            .await
            .unwrap();

        assert_eq!(file_infos.len(), 0);
        assert_eq!(file_set_files.len(), 0);

        // assert that file is marked for deletion for cloud sync (enabled in settings)
        let sync_logs = repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info.id)
            .await
            .unwrap();

        assert!(!sync_logs.is_empty());
        assert_eq!(sync_logs[0].status, FileSyncStatus::DeletionPending);
    }

    #[async_std::test]
    async fn test_update_file_set_unlink_file_from_file_set_linked_to_other_file_set() {
        let repository_manager = create_repository_manager().await;
        let system_id = create_system(&repository_manager).await;
        let settings = create_settings();

        // First, create an existing file set with one file, which we will remove
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
                    original_file_name: existing_file_name.clone(),
                    sha1_checksum: existing_file_checksum,
                    file_size: existing_file_size,
                    archive_file_name: "archive_file_name".to_string(),
                }],
                &[system_id],
            )
            .await
            .unwrap();

        // create another file set with same file
        let _another_file_set_id = repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Another File Set",
                "Another File Set",
                &FileType::DiskImage,
                "Source",
                &[ImportedFile {
                    original_file_name: existing_file_name.clone(),
                    sha1_checksum: existing_file_checksum,
                    file_size: existing_file_size,
                    archive_file_name: "archive_file_name".to_string(),
                }],
                &[system_id],
            )
            .await
            .unwrap();

        let file_info = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(&[existing_file_checksum], FileType::DiskImage)
            .await
            .unwrap()
            .pop()
            .unwrap();

        // simulate a successful cloud sync for the file in file set
        repository_manager
            .get_file_sync_log_repository()
            .add_log_entry(
                file_info.id,
                FileSyncStatus::UploadCompleted,
                "",
                file_info.generate_cloud_key().as_str(),
            )
            .await
            .unwrap();

        let file_import_ops = Arc::new(MockFileImportOps::new());
        let file_metadata_ops = Arc::new(MockFileMetadataOps::new());

        let fs_ops = Arc::new(MockFileSystemOps::new());

        let service = FileImportService {
            repository_manager: repository_manager.clone(),
            fs_ops,
            file_import_ops,
            file_metadata_ops,
            settings,
        };

        let update_file_set_model = UpdateFileSetModel {
            file_set_id,
            // no files are selected, so the existing file will be unlinked from the set
            selected_files: vec![],
            import_files: vec![],
            file_type: FileType::DiskImage,
            // TODO: source should be file specific, not file set specific
            source: "test_source".to_string(),
            file_set_name: "".to_string(),
            file_set_file_name: "".to_string(),
            item_ids: vec![],
            item_types: vec![],
        };

        // Perform the addition
        let result = service.update_file_set(update_file_set_model).await;
        assert!(result.is_ok());

        let file_set_files = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        assert_eq!(file_set_files.len(), 0);

        // Because file was linked to another file set, it should still exist in database
        let file_infos = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(&[existing_file_checksum], FileType::DiskImage)
            .await
            .unwrap();

        assert_eq!(file_infos.len(), 1);

        // assert that file is not marked for deletion for cloud sync (enabled in settings)
        let sync_logs = repository_manager
            .get_file_sync_log_repository()
            .get_logs_by_file_info(file_info.id)
            .await
            .unwrap();

        assert!(!sync_logs.is_empty()); // because we marked it as uploaded before

        let deletion_logs: Vec<_> = sync_logs
            .into_iter()
            .filter(|log| log.status == FileSyncStatus::DeletionPending)
            .collect();
        assert!(deletion_logs.is_empty());
    }

    #[async_std::test]
    async fn test_prepare_import() {
        let repository_manager = create_repository_manager().await;
        let settings = create_settings();

        let sha1_checksum = [0u8; 20];

        let file_in_zip_archive = ReadFile {
            file_name: "test_game.rom".to_string(),
            sha1_checksum,
            file_size: 4096,
        };

        let file_path_str = "/path/to/test_game.zip";
        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(file_path_str);
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let file_metadata_ops = Arc::new(MockFileMetadataOps::new());
        file_metadata_ops.add_zip_file(file_in_zip_archive.sha1_checksum, file_in_zip_archive);

        let service = FileImportService {
            repository_manager,
            fs_ops,
            file_import_ops,
            file_metadata_ops,
            settings,
        };
        let result = service
            .prepare_import(Path::new(file_path_str), FileType::Rom)
            .await;

        assert!(result.is_ok());
        let prepare_result = result.unwrap();
        assert_eq!(
            prepare_result.import_model.path,
            PathBuf::from(file_path_str)
        );
        assert_eq!(
            prepare_result.import_metadata.file_set_name,
            "test_game".to_string()
        );
        assert_eq!(
            prepare_result.import_metadata.file_set_file_name,
            "test_game.zip".to_string()
        );
        assert!(prepare_result.import_metadata.is_zip_archive);

        let import_model_content = prepare_result.import_model.content;
        let imported_file = import_model_content.get(&sha1_checksum).unwrap();
        assert_eq!(imported_file.file_name, "test_game.rom");
        assert_eq!(imported_file.sha1_checksum, sha1_checksum);
    }
}
