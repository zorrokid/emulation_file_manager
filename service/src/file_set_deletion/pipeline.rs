use crate::{
    file_set_deletion::{
        context::DeletionContext,
        steps::{
            DeleteFileInfosStep, DeleteFileSetStep, DeleteLocalFilesStep, FetchFileInfosStep,
            FilterDeletableFilesStep, MarkForCloudDeletionStep, ValidateNotInUseStep,
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<DeletionContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateNotInUseStep),
            Box::new(FetchFileInfosStep),
            Box::new(FilterDeletableFilesStep),
            Box::new(DeleteFileSetStep),
            Box::new(MarkForCloudDeletionStep),
            Box::new(DeleteLocalFilesStep),
            Box::new(DeleteFileInfosStep),
        ])
    }
}
