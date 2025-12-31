use crate::{
    file_import::{
        common_steps::{check_existing_files::CheckExistingFilesStep, import::ImportFilesStep},
        update_file_set::{
            context::UpdateFileSetContext,
            steps::{
                FetchFileSetStep, MarkFilesForCloudSyncStep, RemovedFilesStep,
                UpdateFileInfoToDatabaseStep, UpdateFileSetStep,
            },
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<UpdateFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(FetchFileSetStep),
            Box::new(CheckExistingFilesStep::<UpdateFileSetContext>::new()),
            Box::new(ImportFilesStep::<UpdateFileSetContext>::new()),
            Box::new(UpdateFileInfoToDatabaseStep),
            Box::new(RemovedFilesStep),
            Box::new(UpdateFileSetStep),
            Box::new(MarkFilesForCloudSyncStep),
        ])
    }
}
