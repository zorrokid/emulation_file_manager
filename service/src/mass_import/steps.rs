use crate::{
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
        println!("Importing file sets...");
        let import_items = context.get_import_items();
        println!("Number of import items: {}", import_items.len());
        for item in import_items {
            println!(
                "Importing file set: {:?}",
                item.file_set.as_ref().map(|fs| &fs.file_set_name)
            );
            if let Some(file_set) = item.file_set {
                println!("Creating file set: {}", file_set.file_set_name);
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
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use core_types::{ReadFile, sha1_from_hex_string};
    use dat_file_parser::{
        DatFile, DatFileParserError, DatFileParserOps, DatGame, DatHeader, DatRom, MockDatParser,
    };
    use file_metadata::{FileMetadataError, FileMetadataReader, MockFileMetadataReader};

    use crate::{
        file_import::file_import_service_ops::{
            CreateMockState, FileImportServiceOps, MockFileImportServiceOps,
        },
        file_system_ops::{FileSystemOps, SimpleDirEntry, mock::MockFileSystemOps},
        mass_import::{
            context::{MassImportOps, SendReaderFactoryFn},
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

    #[async_std::test]
    async fn test_import_dat_file_step() {
        let parse_result: Result<DatFile, DatFileParserError> = Ok(DatFile {
            header: DatHeader::default(),
            games: vec![],
        });
        let dat_file_parser_ops = Arc::new(MockDatParser::new(parse_result));

        let mut context = MassImportContext::new(
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
