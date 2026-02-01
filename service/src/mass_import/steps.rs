use crate::{
    error::Error,
    mass_import::context::{FileSetImportResult, FileSetImportStatus, MassImportContext},
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
        let import_items = context.get_import_items();
        for item in import_items {
            if let Some(file_set) = item.file_set {
                let import_res = context
                    .ops
                    .file_import_service_ops
                    .create_file_set(file_set);
                match import_res.await {
                    Ok(import_result) => {
                        if import_result.failed_steps.is_empty() {
                            tracing::info!(
                                file_set_id = %import_result.file_set_id,
                                "Successfully imported file set",
                            );
                            context.state.import_results.push(FileSetImportResult {
                                file_set_id: Some(import_result.file_set_id),
                                status: FileSetImportStatus::Success,
                            });
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
                            context.state.import_results.push(FileSetImportResult {
                                file_set_id: Some(import_result.file_set_id),
                                status: FileSetImportStatus::SucessWithWarnings(errors),
                            });
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            "Failed to import file set",
                        );
                        context.state.import_results.push(FileSetImportResult {
                            file_set_id: None,
                            status: FileSetImportStatus::Failed(format!("{}", e)),
                        });
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
        path::{Path, PathBuf},
        sync::Arc,
    };

    use core_types::ReadFile;
    use dat_file_parser::{
        DatFile, DatFileParserError, DatFileParserOps, DatHeader, MockDatParser,
    };
    use database::{repository_manager::RepositoryManager, setup_test_db};
    use file_metadata::{
        FileMetadataError, FileMetadataReader, MockFileMetadataReader, create_mock_factory,
    };

    use crate::{
        file_import::file_import_service_ops::MockFileImportServiceOps,
        file_system_ops::{FileSystemOps, SimpleDirEntry, mock::MockFileSystemOps},
        mass_import::{
            context::{MassImportDependencies, MassImportOps},
            models::MassImportInput,
        },
    };

    use super::*;

    pub fn create_mock_reader_factory()
    -> impl Fn(&Path) -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
        let mock_metadata = vec![ReadFile {
            file_name: "mock_file.bin".to_string(),
            sha1_checksum: [2u8; 20],
            file_size: 789,
        }];

        let mock_reader = MockFileMetadataReader {
            metadata: mock_metadata.clone(),
        };

        create_mock_factory(mock_reader)
    }

    async fn get_deps() -> MassImportDependencies {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let settings = Arc::new(crate::view_models::Settings::default());

        MassImportDependencies {
            repository_manager,
            settings,
        }
    }

    fn get_ops(
        dat_file_parser_ops: Option<Box<dyn DatFileParserOps>>,
        fs_ops: Option<Box<dyn FileSystemOps>>,
    ) -> MassImportOps {
        let file_import_service_ops = Box::new(MockFileImportServiceOps::new());
        let reader_factory_fn = Box::new(create_mock_reader_factory());
        let parse_result: Result<DatFile, DatFileParserError> = Ok(DatFile {
            header: DatHeader::default(),
            games: vec![],
        });
        let dat_file_parser_ops =
            dat_file_parser_ops.unwrap_or(Box::new(MockDatParser::new(parse_result)));
        let fs_ops = fs_ops.unwrap_or(Box::new(MockFileSystemOps::new()));
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
        let dat_file_parser_ops = Box::new(MockDatParser::new(parse_result));

        let mut context = MassImportContext::with_ops(
            MassImportInput {
                source_path: PathBuf::from("/path/to/source"),
                dat_file_path: Some(PathBuf::from("/path/to/datfile.dat")),
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            get_ops(Some(dat_file_parser_ops), None),
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

        let ops = get_ops(None, Some(Box::new(mock_fs_ops)));

        let mut context = MassImportContext::with_ops(
            MassImportInput {
                source_path: PathBuf::from("/mock"),
                dat_file_path: None,
                file_type: core_types::FileType::Rom,
                item_type: None,
                system_id: 1,
            },
            ops,
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
}
