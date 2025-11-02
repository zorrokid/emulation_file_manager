use crate::{
    file_set_deletion::{
        context::DeletionContext,
        steps::{
            DeleteFileSetStep, DeleteLocalFilesStep, FetchFileInfosStep, FilterDeletableFilesStep,
            MarkForCloudDeletionStep, ValidateNotInUseStep,
        },
    },
    file_system_ops::FileSystemOps,
    pipeline::generic_pipeline::Pipeline,
};

impl<F: FileSystemOps> Pipeline<DeletionContext<F>> {
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
