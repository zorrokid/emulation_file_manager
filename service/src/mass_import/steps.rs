use std::{collections::HashMap, path::PathBuf};

use core_types::{Sha1Checksum, sha1_from_hex_string};

use crate::{
    error::Error,
    file_import::model::{FileImportSource, FileSetImportModel, ImportFileContent},
    mass_import::context::{ImportItem, MassImportContext},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ImportDatFileStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for ImportDatFileStep {
    fn name(&self) -> &'static str {
        "import_dat_file_step"
    }

    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.dat_file_path.is_some()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        let dat_path = context
            .dat_file_path
            .as_ref()
            .expect("Dat file path should be present");

        let parse_res = context.dat_file_parser_ops.parse_dat_file(dat_path);
        match parse_res {
            Ok(dat_file) => {
                println!("Successfully parsed DAT file: {:?}", dat_file);
                context.dat_file = Some(dat_file);
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
        let files_res = context.fs_ops.read_dir(context.source_path.as_path());
        let files = match files_res {
            Ok(files) => files,
            Err(e) => {
                return StepAction::Abort(Error::IoError(format!(
                    "Failed to read source path {}: {}",
                    context.source_path.display(),
                    e
                )));
            }
        };

        for file_res in files {
            match file_res {
                Ok(file) => {
                    tracing::info!("Found file: {}", file.path.display());
                    context.files.push(file.path.clone());
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        path = %context.source_path.display(),
                        "Failed to read a file entry"
                    );
                    context.failed_files.push(context.source_path.clone());
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
            let reader_res = (context.reader_factory_fn)(file);
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
                            context.file_metadata.insert(file.clone(), metadata_entries);
                        }
                        Err(e) => {
                            tracing::error!(
                                error = ?e,
                                file = %file.display(),
                                "Failed to read metadata",
                            );
                            context.failed_files.push(file.clone());
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        file = %file.display(),
                        "Failed to create metadata reader",
                    );
                    context.failed_files.push(file.clone());
                }
            }
        }
        StepAction::Continue
    }
}

// TODO: import file sets and create software titles and releases
// - add file set needs optional create software title and release flags
// - better do that at the same time so rollback is easier if something fails
// - 1. write files to storage
// - 2. if ok create file set, software title, release in a transaction
// - 3. if ok commit transaction otherwise delete written files

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

        StepAction::Continue
    }
}
