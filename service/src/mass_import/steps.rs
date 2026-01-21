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
                    println!("Found file: {}", file.path.display());
                    // Further processing can be done here
                }
                Err(e) => {
                    return StepAction::Abort(Error::IoError(format!(
                        "Error reading file entry: {}",
                        e
                    )));
                }
            }
        }

        // Implementation for reading files goes here
        StepAction::Continue
    }
}
