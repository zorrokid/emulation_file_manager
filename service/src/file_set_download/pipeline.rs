use crate::{
    error::Error,
    file_set_download::{
        context::DownloadContext,
        steps::{
            ConnectToCloudStep, DownloadFilesStep, ExportFilesStep, FetchFileInfoStep,
            PrepareFileForDownloadStep,
        },
    },
    pipeline::{Pipeline, StepAction},
};

// TODO: generalize so that steps can be provided when constructing the pipeline and context is
// generic
impl Pipeline<DownloadContext> {
    pub fn new() -> Self {
        Self {
            steps: vec![
                Box::new(FetchFileInfoStep),
                Box::new(PrepareFileForDownloadStep),
                Box::new(ConnectToCloudStep),
                Box::new(DownloadFilesStep),
                Box::new(ExportFilesStep),
            ],
        }
    }

    pub async fn execute(&self, context: &mut DownloadContext) -> Result<(), Error> {
        for step in &self.steps {
            // Check if step should execute
            if !step.should_execute(context) {
                eprintln!("Skipping step: {}", step.name());
                continue;
            }

            eprintln!("Executing step: {}", step.name());

            match step.execute(context).await {
                StepAction::Continue => {
                    // Proceed to next step
                    continue;
                }
                StepAction::Skip => {
                    // Early successful exit
                    eprintln!("Step {} requested skip - stopping pipeline", step.name());
                    return Ok(());
                }
                StepAction::Abort(error) => {
                    // Error exit
                    eprintln!("Step {} requested abort - stopping pipeline", step.name());
                    return Err(error);
                }
            }
        }

        Ok(())
    }
}
