use crate::{
    cloud_sync::{
        context::SyncContext,
        steps::{
            ConnectToCloudStep, DeleteMarkedFilesStep, PrepareFilesForUploadStep,
            UploadPendingFilesStep,
        },
    },
    error::Error,
    pipeline::{Pipeline, StepAction},
};

impl Default for Pipeline<SyncContext> {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline<SyncContext> {
    pub fn new() -> Self {
        Self {
            steps: vec![
                Box::new(PrepareFilesForUploadStep),
                Box::new(ConnectToCloudStep),
                Box::new(UploadPendingFilesStep),
                Box::new(DeleteMarkedFilesStep),
            ],
        }
    }

    pub async fn execute(&self, context: &mut SyncContext) -> Result<(), Error> {
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
