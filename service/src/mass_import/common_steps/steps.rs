use crate::{
    error::Error,
    mass_import::{
        common_steps::context::MassImportContextOps,
        models::{FileSetImportResult, FileSetImportStatus},
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ReadFilesStep<T: MassImportContextOps> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: MassImportContextOps> Default for ReadFilesStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MassImportContextOps> ReadFilesStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: MassImportContextOps + Send + Sync> PipelineStep<T> for ReadFilesStep<T> {
    fn name(&self) -> &'static str {
        "read_files_step"
    }
    async fn execute(&self, context: &mut T) -> StepAction {
        let files_res = context.fs_ops().read_dir(context.source_path());
        tracing::info!(
            source_path = %context.source_path().display(),
            "Reading files from source path",
        );

        let files = match files_res {
            Ok(files) => files,

            Err(e) => {
                tracing::error!(
                    error = ?e,
                    path = %context.source_path().display(),
                    "Failed to read source path",
                );
                return StepAction::Abort(Error::IoError(format!(
                    "Failed to read source path {}: {}",
                    context.source_path().display(),
                    e
                )));
            }
        };

        for file_res in files {
            tracing::info!(
                source_path = %context.source_path().display(),
                "Processing file entry from source path",
            );
            match file_res {
                Ok(file) => {
                    tracing::info!(
                        file_path = %file.path.display(),
                        "Successfully read file entry from source path",
                    );
                    tracing::info!("Found file: {}", file.path.display());
                    context.read_ok_files_mut().push(file.path.clone());
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        path = %context.source_path().display(),
                        "Failed to read a file entry"
                    );
                    context.dir_scan_errors().push(e);
                }
            }
        }

        // Implementation for reading files goes here
        StepAction::Continue
    }
}

pub struct ReadFileMetadataStep<T: MassImportContextOps> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: MassImportContextOps> Default for ReadFileMetadataStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MassImportContextOps> ReadFileMetadataStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: MassImportContextOps + Send + Sync> PipelineStep<T> for ReadFileMetadataStep<T> {
    fn name(&self) -> &'static str {
        "read_file_metadata_step"
    }

    fn should_execute(&self, context: &T) -> bool {
        !context.get_non_failed_files().is_empty()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        tracing::info!(
            len = %context.get_non_failed_files().len(),
            "Reading metadata for files...",
        );
        for file in &mut context.get_non_failed_files() {
            tracing::info!("Creating metadata reader for file: {}", file.display());
            let reader_res = (context.reader_factory_fn())(file);
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
                                .file_metadata()
                                .insert(file.clone(), metadata_entries);
                        }
                        Err(e) => {
                            tracing::error!(
                                error = ?e,
                                file = %file.display(),
                                "Failed to read metadata",
                            );
                            context.read_failed_files_mut().push(file.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        file = %file.display(),
                        "Failed to create metadata reader",
                    );
                    context.read_failed_files_mut().push(file.clone());
                }
            }
        }
        StepAction::Continue
    }
}

pub struct ImportFileSetsStep<T: MassImportContextOps> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: MassImportContextOps> Default for ImportFileSetsStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: MassImportContextOps> ImportFileSetsStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: MassImportContextOps + Send + Sync> PipelineStep<T> for ImportFileSetsStep<T> {
    fn name(&self) -> &'static str {
        "import_file_sets_step"
    }

    fn should_execute(&self, context: &T) -> bool {
        !context.get_import_file_sets().is_empty()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        let import_file_sets = context.get_import_file_sets();
        for file_set in import_file_sets {
            tracing::info!(
                 file_set_name = %file_set.file_set_name,
                "Creating file set for import",
            );
            let file_set_name = file_set.file_set_name.clone();
            let service = context.import_service_ops();
            let import_res = service.create_file_set(file_set);
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

            context.import_results().push(FileSetImportResult {
                file_set_id: id,
                status: status.clone(),
            });

            if let Some(sender_tx) = &context.progress_tx() {
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

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use async_std::channel::Sender;
    use core_types::{ReadFile, sha1_from_hex_string};
    use dat_file_parser::{DatFileParserError, DatFileParserOps, MockDatParser};
    use file_metadata::SendReaderFactoryFn;

    use super::*;
    use crate::{
        error::Error,
        file_import::{
            file_import_service_ops::{
                CreateMockState, FileImportServiceOps, MockFileImportServiceOps,
            },
            model::{FileImportSource, FileSetImportModel, ImportFileContent},
        },
        file_set::mock_file_set_service::MockFileSetService,
        file_system_ops::{FileSystemOps, SimpleDirEntry, mock::MockFileSystemOps},
        mass_import::{
            common_steps::context::MassImportDeps,
            models::{FileSetImportResult, MassImportInput, MassImportSyncEvent},
            test_utils::create_mock_reader_factory,
            with_dat::context::DatFileMassImportOps,
        },
    };

    struct TestMassImportContext {
        state: TestMassImportState,
        ops: DatFileMassImportOps,
        deps: MassImportDeps,
        input: MassImportInput,
    }

    #[derive(Default, Debug)]
    pub struct TestMassImportState {
        pub read_ok_files: Vec<PathBuf>,
        pub read_failed_files: Vec<PathBuf>,
        pub dir_scan_errors: Vec<Error>,
        pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
        pub import_results: Vec<FileSetImportResult>,
        pub file_sets_to_import: Vec<FileSetImportModel>,
    }

    impl TestMassImportContext {
        pub fn new(
            deps: MassImportDeps,
            input: MassImportInput,
            ops: DatFileMassImportOps,
            state: Option<TestMassImportState>,
        ) -> Self {
            TestMassImportContext {
                state: state.unwrap_or_default(),
                ops,
                deps,
                input,
            }
        }
    }

    async fn get_deps() -> MassImportDeps {
        MassImportDeps {
            repository_manager: database::setup_test_repository_manager().await,
        }
    }

    fn get_ops(
        dat_file_parser_ops: Option<Arc<dyn DatFileParserOps>>,
        fs_ops: Option<Arc<dyn FileSystemOps>>,
        reader_factory_fn: Option<Arc<SendReaderFactoryFn>>,
        file_import_ops: Option<Arc<dyn FileImportServiceOps>>,
    ) -> DatFileMassImportOps {
        let file_import_service_ops =
            file_import_ops.unwrap_or_else(|| Arc::new(MockFileImportServiceOps::new()));
        let parse_result: Result<dat_file_parser::DatFile, DatFileParserError> =
            Ok(dat_file_parser::DatFile {
                header: dat_file_parser::DatHeader::default(),
                games: vec![],
            });
        let dat_file_parser_ops =
            dat_file_parser_ops.unwrap_or(Arc::new(MockDatParser::new(parse_result)));
        let fs_ops = fs_ops.unwrap_or(Arc::new(MockFileSystemOps::new()));
        let reader_factory_fn = reader_factory_fn
            .unwrap_or(Arc::new(create_mock_reader_factory(HashMap::new(), vec![])));
        let file_set_service_ops = Arc::new(MockFileSetService::new());
        DatFileMassImportOps {
            fs_ops,
            file_import_service_ops,
            reader_factory_fn,
            dat_file_parser_ops,
            file_set_service_ops,
        }
    }

    impl MassImportContextOps for TestMassImportContext {
        fn reader_factory_fn(&self) -> Arc<SendReaderFactoryFn> {
            self.ops.reader_factory_fn.clone()
        }

        fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
            self.ops.fs_ops.clone()
        }

        fn source_path(&self) -> &std::path::Path {
            &self.input.source_path
        }

        fn read_ok_files_mut(&mut self) -> &mut Vec<PathBuf> {
            &mut self.state.read_ok_files
        }

        fn read_ok_files(&self) -> &Vec<PathBuf> {
            &self.state.read_ok_files
        }

        fn read_failed_files(&self) -> &Vec<PathBuf> {
            &self.state.read_failed_files
        }

        fn read_failed_files_mut(&mut self) -> &mut Vec<PathBuf> {
            &mut self.state.read_failed_files
        }

        fn dir_scan_errors(&mut self) -> &mut Vec<Error> {
            &mut self.state.dir_scan_errors
        }

        fn file_metadata(&mut self) -> &mut HashMap<PathBuf, Vec<ReadFile>> {
            &mut self.state.file_metadata
        }

        fn get_import_file_sets(&self) -> Vec<FileSetImportModel> {
            self.state.file_sets_to_import.clone()
        }

        fn import_service_ops(&self) -> Arc<dyn FileImportServiceOps> {
            self.ops.file_import_service_ops.clone()
        }

        fn import_results(&mut self) -> &mut Vec<FileSetImportResult> {
            &mut self.state.import_results
        }

        fn progress_tx(&self) -> &Option<Sender<MassImportSyncEvent>> {
            &None
        }
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

        let mut context = TestMassImportContext::new(
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

        let step = ReadFilesStep::<TestMassImportContext>::new();
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
        let mut context = TestMassImportContext::new(
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

        let step = ReadFileMetadataStep::<TestMassImportContext>::new();
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

        // file metadata that has been read from file
        let file_checksum =
            sha1_from_hex_string("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
        let file = ReadFile {
            file_name: "rom1.bin".to_string(),
            sha1_checksum: file_checksum,
            file_size: 123,
        };
        let file_path = PathBuf::from("/mock/rom1.zip");

        // File metadata matching the ROM SHA1
        let mut file_metadata = HashMap::new();
        file_metadata.insert(file_path.clone(), vec![file.clone()]);

        // Mock FileImportServiceOps with deterministic output
        let mock_file_import_ops = MockFileImportServiceOps::with_create_mock(CreateMockState {
            file_set_id: 42,
            release_id: Some(7),
        });

        let ops = get_ops(None, None, None, Some(Arc::new(mock_file_import_ops)));
        let mut context = TestMassImportContext::new(
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
        context.state.file_metadata = file_metadata;

        let import_file_content = ImportFileContent {
            file_name: file.file_name.clone(),
            sha1_checksum: file.sha1_checksum,
            file_size: file.file_size,
        };

        // File sets to import are compiled for example based on the DAT file data and file metadata
        context.state.file_sets_to_import.push(FileSetImportModel {
            file_set_name: "Test Game".to_string(),
            file_set_file_name: "Test Game".to_string(),
            import_files: vec![FileImportSource {
                path: file_path.clone(),
                content: HashMap::from([(file_checksum, import_file_content)]),
            }],
            selected_files: vec![file_checksum],
            system_ids: vec![context.input.system_id],
            source: "dat".to_string(),
            file_type: context.input.file_type,
            item_ids: vec![],
            item_types: vec![],
            create_release: None,
            dat_file_id: None,
        });

        context
            .state
            .read_ok_files
            .push(PathBuf::from("/mock/rom1.zip"));

        // Act
        let step = ImportFileSetsStep::<TestMassImportContext>::new();
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
