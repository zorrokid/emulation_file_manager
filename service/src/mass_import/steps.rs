use crate::{
    error::Error,
    mass_import::context::{ImportItem, ImportItemStatus, MassImportContext},
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
                    context.import_items.push(ImportItem {
                        path: file.path.clone(),
                        status: ImportItemStatus::Pending,
                        release_name: String::new(),
                        software_title_name: String::new(),
                        file_set: None,
                    });
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

pub struct CheckFilesStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for CheckFilesStep {
    fn name(&self) -> &'static str {
        "check_files_step"
    }

    fn should_execute(&self, context: &MassImportContext) -> bool {
        !context.import_items.is_empty()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        // Implementation for checking files goes here
        println!(
            "Checking files in source path: {}",
            context.source_path.display()
        );
        for import_item in &mut context.import_items {
            let file = &import_item.path;
            println!("Checking file: {}", file.display());
            let reader_res = (context.reader_factory_fn)(file);
            match reader_res {
                Ok(reader) => {
                    println!(
                        "Successfully created metadata reader for file: {}",
                        file.display()
                    );
                    let res = reader.read_metadata();
                    println!("Metadata for file {}: {:?}", file.display(), res);
                }
                Err(e) => {
                    println!(
                        "Failed to create metadata reader for file {}: {}",
                        file.display(),
                        e
                    );
                    import_item.status = ImportItemStatus::Failed(format!("Reader error: {}", e));
                }
            }
        }
        StepAction::Continue
    }
}

pub struct CollectExistingFilesStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for CollectExistingFilesStep {
    fn name(&self) -> &'static str {
        "collect_existing_files_step"
    }
    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.dat_file.is_some() && !context.import_items.is_empty()
    }
    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        // Implementation for collecting existing files goes here
        println!("Collecting existing files...");
        // TODO: check each file if they exist in the database
        StepAction::Continue
    }
}
pub struct CollectFilesMatchingDatFileStep;

#[async_trait::async_trait]
impl PipelineStep<MassImportContext> for CollectFilesMatchingDatFileStep {
    fn name(&self) -> &'static str {
        "collect_files_matching_dat_file_step"
    }
    fn should_execute(&self, context: &MassImportContext) -> bool {
        context.dat_file.is_some() && !context.import_items.is_empty()
    }
    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        // Implementation for collecting matching files goes here
        println!("Collecting matching files...");
        // TODO: check each file against the dat file entries
        // get file set name and name for each from dat file
        StepAction::Continue
    }
}

pub struct CreateFileSetsStep;

pub struct CreateReleasesStep;
