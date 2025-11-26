use crate::{
    external_executable_runner::context::ExternalExecutableRunnerContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct PrepareFilesStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for PrepareFilesStep {
    fn name(&self) -> &'static str {
        "prepare_files"
    }

    async fn execute(&self, _context: &mut ExternalExecutableRunnerContext) -> StepAction {
        // Implementation for preparing files goes here
        StepAction::Continue
    }
}

pub struct StartExecutableStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for StartExecutableStep {
    fn name(&self) -> &'static str {
        "start_executable"
    }

    async fn execute(&self, _context: &mut ExternalExecutableRunnerContext) -> StepAction {
        // Implementation for starting the executable goes here
        StepAction::Continue
    }
}

pub struct CleanupFilesStep;

#[async_trait::async_trait]
impl PipelineStep<ExternalExecutableRunnerContext> for CleanupFilesStep {
    fn name(&self) -> &'static str {
        "cleanup_files"
    }

    async fn execute(&self, _context: &mut ExternalExecutableRunnerContext) -> StepAction {
        // Implementation for cleaning up files goes here
        StepAction::Continue
    }
}
