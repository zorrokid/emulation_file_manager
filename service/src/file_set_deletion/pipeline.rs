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
};

/// Result of executing a pipeline step
#[derive(Debug, Clone, PartialEq)]
pub enum StepAction {
    /// Continue to the next step
    Continue,
    /// Skip all remaining steps (successful early exit)
    Skip,
    /// Abort the pipeline with an error
    Abort(Error),
}

/// Trait for pipeline steps in the deletion process
#[async_trait::async_trait]
pub trait DeletionStep<F: FileSystemOps>: Send + Sync {
    fn name(&self) -> &'static str;

    /// Determines if this step should execute based on current context
    fn should_execute(&self, _context: &DeletionContext<F>) -> bool {
        true // By default, always execute
    }

    /// Execute the step, modifying the context and returning the next action
    async fn execute(&self, context: &mut DeletionContext<F>) -> StepAction;
}

pub struct DeletionPipeline<F: FileSystemOps> {
    steps: Vec<Box<dyn DeletionStep<F>>>,
}

impl<F: FileSystemOps> DeletionPipeline<F> {
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
