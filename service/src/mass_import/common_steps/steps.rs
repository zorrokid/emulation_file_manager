use crate::{
    error::Error,
    mass_import::common_steps::context::MassImportContextOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

/// Step to read files from the source path and populate context with read_ok_files and
/// read_failed_files based on whether each file was read successfully or not. Also populates
/// dir_scan_errors in case of any errors while reading the source directory.
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

/// Step to read metadata for files read in the previous step and populate context with
/// file_metadata for successfully read metadata and read_failed_files for files whose metadata
/// could not be read for any reason.
///
/// For example if the file is a zip archive, metadata of each
/// file in the zip archive is read and stored in the context. If the file is a regular file,
/// metadata of the file is read and stored in the context.
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
        !context.read_ok_files().is_empty()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        tracing::info!("Reading metadata for files.",);
        for file in &context.get_non_failed_files() {
            tracing::info!("Creating metadata reader for file: {}", file.display());
            let reader_res = (context.reader_factory_fn())(file);
            match reader_res {
                Ok(reader) => {
                    tracing::info!(
                        file = %file.display(),
                        "Successfully created metadata reader",
                    );
                    let res = reader.read_metadata();
                    match res {
                        Ok(metadata_entries) => {
                            tracing::info!(
                                file = %file.display(),
                                metadata_entries = ?metadata_entries,
                                "Successfully read metadata",
                            );
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

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::ReadFile;
    use dat_file_parser::{DatFileParserError, DatFileParserOps, MockDatParser};
    use file_metadata::SendReaderFactoryFn;
    use flume::Sender;

    use super::*;
    use crate::{
        error::Error,
        file_import::file_import_service_ops::{FileImportServiceOps, MockFileImportServiceOps},
        file_set::mock_file_set_service::MockFileSetService,
        file_system_ops::{FileSystemOps, SimpleDirEntry, mock::MockFileSystemOps},
        mass_import::{
            common_steps::context::CommonMassImportState,
            models::{DatMassImportInput, MassImportSyncEvent},
            test_utils::create_mock_reader_factory,
            with_dat::context::DatFileMassImportOps,
        },
    };

    struct TestMassImportContext {
        state: TestMassImportState,
        ops: DatFileMassImportOps,
        input: DatMassImportInput,
    }

    #[derive(Default, Debug)]
    pub struct TestMassImportState {
        pub common_state: CommonMassImportState,
    }

    impl TestMassImportContext {
        pub fn new(
            input: DatMassImportInput,
            ops: DatFileMassImportOps,
            state: Option<TestMassImportState>,
        ) -> Self {
            TestMassImportContext {
                state: state.unwrap_or_default(),
                ops,
                input,
            }
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
        fn common_state(&self) -> &CommonMassImportState {
            &self.state.common_state
        }

        fn common_state_mut(&mut self) -> &mut CommonMassImportState {
            &mut self.state.common_state
        }

        fn reader_factory_fn(&self) -> Arc<SendReaderFactoryFn> {
            self.ops.reader_factory_fn.clone()
        }

        fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
            self.ops.fs_ops.clone()
        }

        fn source_path(&self) -> &std::path::Path {
            &self.input.source_path
        }

        fn import_service_ops(&self) -> Arc<dyn FileImportServiceOps> {
            self.ops.file_import_service_ops.clone()
        }

        fn progress_tx(&self) -> &Option<Sender<MassImportSyncEvent>> {
            &None
        }
    }

    #[async_std::test]
    async fn test_read_files_step() {
        // Prepare mock file system ops to return two files
        let mock_fs_ops = MockFileSystemOps::new();
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
            DatMassImportInput {
                source_path: PathBuf::from("/mock"),
                dat_file_path: PathBuf::from("/dummy.dat"),
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
        assert_eq!(context.state.common_state.read_ok_files.len(), 2);
        assert_eq!(context.state.common_state.dir_scan_errors.len(), 1);
        assert!(
            context
                .state
                .common_state
                .read_ok_files
                .contains(&PathBuf::from(&file1))
        );
        assert!(
            context
                .state
                .common_state
                .read_ok_files
                .contains(&PathBuf::from(&file2))
        );
        assert!(
            context
                .state
                .common_state
                .dir_scan_errors
                .contains(&entry3_error)
        );
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
            DatMassImportInput {
                source_path: PathBuf::from("/mock"),
                dat_file_path: PathBuf::from("/dummy.dat"),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            ops,
            None,
        );

        context
            .state
            .common_state
            .read_ok_files
            .push(PathBuf::from("/mock/file1.zip"));
        context
            .state
            .common_state
            .read_ok_files
            .push(PathBuf::from("/mock/file2.zip"));

        let step = ReadFileMetadataStep::<TestMassImportContext>::new();
        let res = step.execute(&mut context).await;
        assert!(matches!(res, StepAction::Continue));
        assert_eq!(context.state.common_state.file_metadata.len(), 2);
        assert!(
            context
                .state
                .common_state
                .file_metadata
                .contains_key(&PathBuf::from("/mock/file1.zip"))
        );
        assert!(
            context
                .state
                .common_state
                .file_metadata
                .contains_key(&PathBuf::from("/mock/file2.zip"))
        );
        assert!(context.state.common_state.read_failed_files.is_empty());
        let file_1_metadata = context
            .state
            .common_state
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
            .common_state
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
                .common_state
                .read_failed_files
                .contains(&PathBuf::from("/mock/file1.zip"))
        );
    }
}
