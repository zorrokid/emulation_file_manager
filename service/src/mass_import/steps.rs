use crate::{
    dat_file_service::DatFileService,
    error::Error,
    mass_import::{
        context::MassImportContext,
        models::{FileSetImportResult, FileSetImportStatus},
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ImportDatFileStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ImportDatFileStep {
    fn name(&self) -> &'static str {
        "import_dat_file_step"
    }

    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.input.dat_file_path.is_some()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let dat_path = context
            .input
            .dat_file_path
            .as_ref()
            .expect("Dat file path should be present");

        let parse_res = context.ops.dat_file_parser_ops.parse_dat_file(dat_path);
        match parse_res {
            Ok(dat_file) => {
                println!("Successfully parsed DAT file: {:?}", dat_file);
                context.state.dat_file = Some(dat_file);
            }
            Err(e) => {
                // Abort since dat file was explicitly provided
                return StepAction::Abort(Error::ParseError(format!(
                    "Failed to parse DAT file {}: {}",
                    dat_path.display(),
                    e
                )));
            }
        }

        StepAction::Continue
    }
}

pub struct CheckExistingDatFileStep;
#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for CheckExistingDatFileStep {
    fn name(&self) -> &'static str {
        "check_existing_dat_file_step"
    }
    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.state.dat_file.is_some()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let dat_file = context
            .state
            .dat_file
            .as_ref()
            .expect("DAT file should be present in state");
        let is_existing_dat_res = context
            .deps
            .repository_manager
            .get_dat_repository()
            .check_dat_file_exists(
                dat_file.header.version.as_str(),
                dat_file.header.name.as_str(), // TODO: use dat file type instead?
                context.input.system_id,
            )
            .await;

        match is_existing_dat_res {
            Ok(id_res) => {
                if let Some(id) = id_res {
                    tracing::info!(
                        system_id = context.input.system_id,
                        dat_name = %dat_file.header.name,
                        dat_version = %dat_file.header.version,
                        "DAT file already exists in the database",
                    );
                    context.state.dat_file_id = Some(id);
                } else {
                    tracing::info!(
                        system_id = context.input.system_id,
                        dat_name = %dat_file.header.name,
                        dat_version = %dat_file.header.version,
                        "DAT file does not exist in the database, proceeding to store it",
                    );
                }
                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    system_id = context.input.system_id,
                    dat_name = %dat_file.header.name,
                    dat_version = %dat_file.header.version,
                    error = ?err,
                    "Error while checking if DAT file exists in the database",
                );
                StepAction::Abort(Error::DbError(format!(
                    "Error while checking if DAT file exists in the database: {}",
                    err
                )))
            }
        }
    }
}

pub struct StoreDatFileStep;
#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for StoreDatFileStep {
    fn name(&self) -> &'static str {
        "store_dat_file_step"
    }
    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.state.dat_file.is_some() && context.state.dat_file_id.is_none()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let dat_file = context
            .state
            .dat_file
            .as_ref()
            .expect("DAT file should be present in state");

        let dat_service = DatFileService::new(context.deps.repository_manager.clone());
        match dat_service
            .store_dat_file(dat_file, context.input.system_id)
            .await
        {
            Ok(dat_file_id) => {
                println!("Successfully stored DAT file with ID: {}", dat_file_id);
                context.state.dat_file_id = Some(dat_file_id);
            }
            Err(e) => {
                println!("Failed to store DAT file: {}", e);
                return StepAction::Abort(Error::DbError(format!(
                    "Failed to store DAT file: {}",
                    e
                )));
            }
        }

        StepAction::Continue
    }
}

pub struct ReadFilesStep;
#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ReadFilesStep {
    fn name(&self) -> &'static str {
        "read_files_step"
    }
    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let files_res = context
            .ops
            .fs_ops
            .read_dir(context.input.source_path.as_path());
        println!(
            "Reading files from source path: {}",
            context.input.source_path.display()
        );

        let files = match files_res {
            Ok(files) => files,

            Err(e) => {
                println!(
                    "Failed to read source path {}: {}",
                    context.input.source_path.display(),
                    e
                );
                return StepAction::Abort(Error::IoError(format!(
                    "Failed to read source path {}: {}",
                    context.input.source_path.display(),
                    e
                )));
            }
        };

        for file_res in files {
            println!(
                "Processing file entry from source path: {}",
                context.input.source_path.display()
            );
            match file_res {
                Ok(file) => {
                    println!("Successfully read file entry: {}", file.path.display());
                    tracing::info!("Found file: {}", file.path.display());
                    context.state.read_ok_files.push(file.path.clone());
                }
                Err(e) => {
                    println!(
                        "Failed to read file entry from source path {}: {}",
                        context.input.source_path.display(),
                        e
                    );
                    tracing::error!(
                        error = ?e,
                        path = %context.input.source_path.display(),
                        "Failed to read a file entry"
                    );
                    context.state.dir_scan_errors.push(e);
                }
            }
        }

        // Implementation for reading files goes here
        StepAction::Continue
    }
}

pub struct ReadFileMetadataStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ReadFileMetadataStep {
    fn name(&self) -> &'static str {
        "read_file_metadata_step"
    }

    fn should_execute(&self, context: &MassImportContext) -> bool {
        !context.get_non_failed_files().is_empty()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        tracing::info!(
            len = %context.get_non_failed_files().len(),
            "Reading metadata for files...",
        );
        for file in &mut context.get_non_failed_files() {
            tracing::info!("Creating metadata reader for file: {}", file.display());
            let reader_res = (context.ops.reader_factory_fn)(file);
            match reader_res {
                Ok(reader) => {
                    tracing::info!(
                        file = %file.display(),
                        "Successfully created metadata reader",
                    );
                    let res = reader.read_metadata();
                    tracing::info!(
                        file = %file.display(),
                        "Successfully read metadata",
                    );
                    match res {
                        Ok(metadata_entries) => {
                            context
                                .state
                                .file_metadata
                                .insert(file.clone(), metadata_entries);
                        }
                        Err(e) => {
                            tracing::error!(
                                error = ?e,
                                file = %file.display(),
                                "Failed to read metadata",
                            );
                            context.state.read_failed_files.push(file.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        file = %file.display(),
                        "Failed to create metadata reader",
                    );
                    context.state.read_failed_files.push(file.clone());
                }
            }
        }
        StepAction::Continue
    }
}

pub struct ImportFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ImportFileSetsStep {
    fn name(&self) -> &'static str {
        "import_file_sets_step"
    }
    fn should_execute(&self, context: &MassImportContext) -> bool {
        !context.get_non_failed_files().is_empty()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let import_items = context.get_import_items();
        for item in import_items {
            tracing::info!(
                file_set_name = item.file_set.as_ref().map(|fs| &fs.file_set_name),
                "Importing file set",
            );
            if let Some(file_set) = item.file_set {
                tracing::info!(
                     file_set_name = %file_set.file_set_name,
                    "Creating file set for import",
                );
                let file_set_name = file_set.file_set_name.clone();
                let import_res = context
                    .ops
                    .file_import_service_ops
                    .create_file_set(file_set);
                let (id, status) = match import_res.await {
                    Ok(import_result) => {
                        println!(
                            "Successfully imported file set: {}",
                            import_result.file_set_id
                        );
                        if import_result.failed_steps.is_empty() {
                            tracing::info!(
                                file_set_id = %import_result.file_set_id,
                                "Successfully imported file set",
                            );
                            (
                                Some(import_result.file_set_id),
                                FileSetImportStatus::Success,
                            )
                        } else {
                            tracing::warn!(
                                file_set_id = %import_result.file_set_id,
                                "File set imported with some failed steps",
                            );
                            let errors: Vec<String> = import_result
                                .failed_steps
                                .iter()
                                .map(|(step, error)| format!("{}: {}", step, error))
                                .collect();
                            for (step, error) in import_result.failed_steps {
                                tracing::warn!(
                                    step = %step,
                                    error = %error,
                                    "Failed step in file set import",
                                );
                            }
                            (
                                Some(import_result.file_set_id),
                                FileSetImportStatus::SucessWithWarnings(errors),
                            )
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            "Failed to import file set",
                        );
                        (None, FileSetImportStatus::Failed(format!("{}", e)))
                    }
                };
                println!("Import result for file set {}: {:?}", file_set_name, status);

                context.state.import_results.push(FileSetImportResult {
                    file_set_id: id,
                    status: status.clone(),
                });

                if let Some(sender_tx) = &context.progress_tx {
                    let event = crate::mass_import::models::MassImportSyncEvent {
                        file_set_name,
                        status,
                    };
                    if let Err(e) = sender_tx.send(event).await {
                        tracing::error!(
                            error = ?e,
                            "Failed to send progress event",
                        );
                    }
                }
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::{ReadFile, sha1_from_hex_string};
    use dat_file_parser::{
        DatFile, DatFileParserError, DatFileParserOps, DatGame, DatHeader, DatRom, MockDatParser,
    };
    use database::helper::AddDatFileParams;

    use crate::{
        file_import::file_import_service_ops::{
            CreateMockState, FileImportServiceOps, MockFileImportServiceOps,
        },
        file_system_ops::{FileSystemOps, SimpleDirEntry, mock::MockFileSystemOps},
        mass_import::{
            context::{MassImportDeps, MassImportOps, SendReaderFactoryFn},
            models::MassImportInput,
            test_utils::create_mock_reader_factory,
        },
    };

    use super::*;

    fn get_ops(
        dat_file_parser_ops: Option<Arc<dyn DatFileParserOps>>,
        fs_ops: Option<Arc<dyn FileSystemOps>>,
        reader_factory_fn: Option<Arc<SendReaderFactoryFn>>,
        file_import_ops: Option<Arc<dyn FileImportServiceOps>>,
    ) -> MassImportOps {
        let file_import_service_ops =
            file_import_ops.unwrap_or_else(|| Arc::new(MockFileImportServiceOps::new()));
        let parse_result: Result<DatFile, DatFileParserError> = Ok(DatFile {
            header: DatHeader::default(),
            games: vec![],
        });
        let dat_file_parser_ops =
            dat_file_parser_ops.unwrap_or(Arc::new(MockDatParser::new(parse_result)));
        let fs_ops = fs_ops.unwrap_or(Arc::new(MockFileSystemOps::new()));
        let reader_factory_fn = reader_factory_fn
            .unwrap_or(Arc::new(create_mock_reader_factory(HashMap::new(), vec![])));
        MassImportOps {
            fs_ops,
            file_import_service_ops,
            reader_factory_fn,
            dat_file_parser_ops,
        }
    }

    async fn get_deps() -> MassImportDeps {
        let pool = Arc::new(database::setup_test_db().await);
        let repository_manager =
            Arc::new(database::repository_manager::RepositoryManager::new(pool));
        MassImportDeps { repository_manager }
    }

    #[async_std::test]
    async fn test_import_dat_file_step() {
        let parse_result: Result<DatFile, DatFileParserError> = Ok(DatFile {
            header: DatHeader::default(),
            games: vec![],
        });
        let dat_file_parser_ops = Arc::new(MockDatParser::new(parse_result));

        let mut context = MassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            get_ops(Some(dat_file_parser_ops), None, None, None),
            None,
        );

        let step = ImportDatFileStep;
        let result = step.execute(&mut context).await;
        assert!(matches!(result, StepAction::Continue));
        assert!(context.state.dat_file.is_some());
    }

    #[async_std::test]
    async fn test_check_existing_dat_file_step_with_existing_dat_file() {
        // Arrange

        // Prepare a dat file and add it to the repository to simulate existing dat fil
        let deps = get_deps().await;
        let system_repo = deps.repository_manager.get_system_repository();
        let system_id = system_repo
            .add_system("Test System")
            .await
            .expect("Failed to add test system");
        let dat_repo = deps.repository_manager.get_dat_repository();

        let dat_file = DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![],
        };

        let add_dat_file_params = AddDatFileParams {
            dat_id: dat_file.header.id,
            name: dat_file.header.name.as_str(),
            description: dat_file.header.description.as_str(),
            version: dat_file.header.version.as_str(),
            date: dat_file.header.date.as_deref(),
            author: dat_file.header.author.as_str(),
            homepage: dat_file.header.homepage.as_deref(),
            url: dat_file.header.url.as_deref(),
            subset: dat_file.header.subset.as_deref(),
            system_id,
        };
        dat_repo.add_dat_file(add_dat_file_params).await.unwrap();

        let dat_file_parser_ops = Arc::new(MockDatParser::new(Ok(dat_file.clone())));
        let mut context = MassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            get_ops(Some(dat_file_parser_ops), None, None, None),
            None,
        );

        // Pre-populate state with a dat file to trigger the step
        context.state.dat_file = Some(dat_file);

        // Act
        let result = CheckExistingDatFileStep.execute(&mut context).await;

        // Assert
        assert!(matches!(result, StepAction::Continue));
    }

    #[async_std::test]
    async fn test_read_files_step() {
        // Prepare mock file system ops to return two files
        let mut mock_fs_ops = MockFileSystemOps::new();
        let file1 = String::from("/mock/file1.bin");
        let file2 = String::from("/mock/file2.bin");
        //let file3 = String::from("/mock/file3.bin");
        let entry1: Result<SimpleDirEntry, Error> = Ok(SimpleDirEntry {
            path: PathBuf::from(&file1),
        });
        let entry2: Result<SimpleDirEntry, Error> = Ok(SimpleDirEntry {
            path: PathBuf::from(&file2),
        });
        let entry3_error = Error::IoError("Simulated read failure".to_string());
        let entry3: Result<SimpleDirEntry, Error> = Err(entry3_error.clone()); // Simulate failure for file3

        mock_fs_ops.add_entry(entry1);
        mock_fs_ops.add_entry(entry2);
        mock_fs_ops.add_entry(entry3);

        let ops = get_ops(None, Some(Arc::new(mock_fs_ops)), None, None);

        let mut context = MassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/mock"),
                dat_file_path: None,
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            ops,
            None,
        );

        let step = ReadFilesStep;
        let result = step.execute(&mut context).await;
        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.state.read_ok_files.len(), 2);
        assert_eq!(context.state.dir_scan_errors.len(), 1);
        assert!(context.state.read_ok_files.contains(&PathBuf::from(&file1)));
        assert!(context.state.read_ok_files.contains(&PathBuf::from(&file2)));
        assert!(context.state.dir_scan_errors.contains(&entry3_error));
    }

    #[async_std::test]
    async fn test_read_file_metadata_step() {
        let mut metadata_by_path = HashMap::new();
        metadata_by_path.insert(
            PathBuf::from("/mock/file1.zip"),
            vec![
                ReadFile {
                    file_name: "file1.bin".to_string(),
                    sha1_checksum: [1u8; 20],
                    file_size: 123,
                },
                ReadFile {
                    file_name: "file2.bin".to_string(),
                    sha1_checksum: [3u8; 20],
                    file_size: 456,
                },
            ],
        );
        metadata_by_path.insert(
            PathBuf::from("/mock/file2.zip"),
            vec![ReadFile {
                file_name: "file2.bin".to_string(),
                sha1_checksum: [2u8; 20],
                file_size: 456,
            }],
        );
        metadata_by_path.insert(
            PathBuf::from("/mock/file3.zip"),
            vec![], // This file will simulate failure
        );
        let reader_factory =
            create_mock_reader_factory(metadata_by_path, vec![PathBuf::from("/mock/file3.zip")]);
        let ops = get_ops(None, None, Some(Arc::new(reader_factory)), None);
        let mut context = MassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/mock"),
                dat_file_path: None,
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            ops,
            None,
        );

        context
            .state
            .read_ok_files
            .push(PathBuf::from("/mock/file1.zip"));
        context
            .state
            .read_ok_files
            .push(PathBuf::from("/mock/file2.zip"));

        let step = ReadFileMetadataStep;
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Continue));
        assert_eq!(context.state.file_metadata.len(), 2);
        assert!(
            context
                .state
                .file_metadata
                .contains_key(&PathBuf::from("/mock/file1.zip"))
        );
        assert!(
            context
                .state
                .file_metadata
                .contains_key(&PathBuf::from("/mock/file2.zip"))
        );
        assert!(context.state.read_failed_files.is_empty());
        let file_1_metadata = context
            .state
            .file_metadata
            .get(&PathBuf::from("/mock/file1.zip"))
            .unwrap();
        assert_eq!(file_1_metadata.len(), 2);
        assert_eq!(file_1_metadata[0].file_name, "file1.bin");
        assert_eq!(file_1_metadata[0].file_size, 123);
        assert_eq!(file_1_metadata[0].sha1_checksum, [1u8; 20]);
        assert_eq!(file_1_metadata[1].file_name, "file2.bin");
        assert_eq!(file_1_metadata[1].file_size, 456);
        assert_eq!(file_1_metadata[1].sha1_checksum, [3u8; 20]);
        let file_2_metadata = context
            .state
            .file_metadata
            .get(&PathBuf::from("/mock/file2.zip"))
            .unwrap();
        assert_eq!(file_2_metadata.len(), 1);
        assert_eq!(file_2_metadata[0].file_name, "file2.bin");
        assert_eq!(file_2_metadata[0].file_size, 456);
        assert_eq!(file_2_metadata[0].sha1_checksum, [2u8; 20]);
        assert!(
            !context
                .state
                .read_failed_files
                .contains(&PathBuf::from("/mock/file1.zip"))
        );
    }
    #[async_std::test]
    async fn test_import_file_sets_step_success() {
        // Arrange
        // Minimal DAT with one game and one ROM
        let dat_game = DatGame {
            name: "Test Game".to_string(),
            description: "Test Game".to_string(),
            roms: vec![DatRom {
                name: "rom1.bin".to_string(),
                sha1: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
                size: 123,
                ..Default::default()
            }],
            ..Default::default()
        };
        let dat_file = DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![dat_game],
        };
        // File metadata matching the ROM SHA1
        let mut file_metadata = HashMap::new();
        file_metadata.insert(
            PathBuf::from("/mock/rom1.zip"),
            vec![ReadFile {
                file_name: "rom1.bin".to_string(),
                sha1_checksum: sha1_from_hex_string("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                    .unwrap(),
                file_size: 123,
            }],
        );
        // Mock FileImportServiceOps with deterministic output
        let mock_file_import_ops = MockFileImportServiceOps::with_create_mock(CreateMockState {
            file_set_id: 42,
            release_id: Some(7),
        });
        let ops = get_ops(None, None, None, Some(Arc::new(mock_file_import_ops)));
        let mut context = MassImportContext::new(
            get_deps().await,
            MassImportInput {
                source_path: PathBuf::from("/mock"),
                dat_file_path: None,
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            ops,
            None,
        );
        // Pre-populate state as if previous steps succeeded
        context.state.dat_file = Some(dat_file);
        context.state.file_metadata = file_metadata;
        context
            .state
            .read_ok_files
            .push(PathBuf::from("/mock/rom1.zip"));
        // Act
        let step = ImportFileSetsStep;
        let result = step.execute(&mut context).await;
        // Assert
        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.state.import_results.len(), 1);
        let import_result = &context.state.import_results[0];
        assert_eq!(import_result.file_set_id, Some(42));
        match &import_result.status {
            FileSetImportStatus::Success => {}
            other => panic!("Unexpected import status: {:?}", other),
        }
    }
}
