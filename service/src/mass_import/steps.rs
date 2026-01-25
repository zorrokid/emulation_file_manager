use crate::{
    error::Error,
    mass_import::context::MassImportContext,
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
                    context.files.push(file.path);
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        path = %context.source_path.display(),
                        "Failed to read a file entry"
                    );
                    context.failed_files.push((
                        context.source_path.clone(),
                        format!("Failed to read file entry: {}", e),
                    ));
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
        !context.files.is_empty()
    }

    async fn execute(&self, context: &mut MassImportContext) -> StepAction {
        // Implementation for checking files goes here
        println!(
            "Checking files in source path: {}",
            context.source_path.display()
        );
        for file in &context.files {
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
                    context.failed_files.push((
                        file.clone(),
                        format!("Failed to create metadata reader: {}", e),
                    ));
                }
            }
        }
        StepAction::Continue
    }
}
