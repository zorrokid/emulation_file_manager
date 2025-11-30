use crate::{
    external_executable_runner::{
        context::ExternalExecutableRunnerContext,
        steps::{CleanupFilesStep, PrepareFilesStep, StartExecutableStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<ExternalExecutableRunnerContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(PrepareFilesStep),
            Box::new(StartExecutableStep),
            Box::new(CleanupFilesStep),
        ])
    }
}
