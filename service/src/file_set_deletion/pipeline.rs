use crate::{
    file_import::common_steps::file_deletion_steps::{
        DeleteFileInfosStep, DeleteLocalFilesStep, FilterDeletableFilesStep,
        MarkForCloudDeletionStep,
    },
    file_set_deletion::{
        context::DeletionContext,
        steps::{DeleteFileSetStep, FetchFileInfosStep, ValidateFileSetNotInUseStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<DeletionContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateFileSetNotInUseStep),
            Box::new(FetchFileInfosStep),
            Box::new(FilterDeletableFilesStep::<DeletionContext>::new()),
            Box::new(DeleteFileSetStep),
            Box::new(DeleteLocalFilesStep::<DeletionContext>::new()),
            Box::new(MarkForCloudDeletionStep::<DeletionContext>::new()),
            Box::new(DeleteFileInfosStep::<DeletionContext>::new()),
        ])
    }
}
