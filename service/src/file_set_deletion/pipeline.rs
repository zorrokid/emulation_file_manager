use crate::{
    error::Error,
    file_set_deletion::{
        context::DeletionContext,
        steps::{
            DeleteFileSetStep, DeleteLocalFilesStep, FetchFileInfosStep, FilterDeletableFilesStep,
            MarkForCloudDeletionStep, ValidateNotInUseStep,
        },
    },
    file_system_ops::FileSystemOps,
    pipeline::{Pipeline, StepAction},
};

impl<F: FileSystemOps> Pipeline<DeletionContext<F>> {
    pub fn new() -> Self {
        Self {
            steps: vec![
                Box::new(ValidateNotInUseStep),
                Box::new(FetchFileInfosStep),
                Box::new(DeleteFileSetStep),
                Box::new(FilterDeletableFilesStep),
                Box::new(MarkForCloudDeletionStep),
                Box::new(DeleteLocalFilesStep),
            ],
        }
    }

    pub async fn execute(&self, context: &mut DeletionContext<F>) -> Result<(), Error> {
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
