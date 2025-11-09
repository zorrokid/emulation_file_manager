use crate::{
    file_set_deletion::{
        context::DeletionContext,
        steps::{
            DeleteFileSetStep, DeleteLocalFilesStep, FetchFileInfosStep, FilterDeletableFilesStep,
            MarkForCloudDeletionStep, ValidateNotInUseStep,
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<DeletionContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateNotInUseStep),
            Box::new(FetchFileInfosStep),
            Box::new(DeleteFileSetStep),
            Box::new(FilterDeletableFilesStep),
            Box::new(MarkForCloudDeletionStep),
            Box::new(DeleteLocalFilesStep),
        ])
    }
}
