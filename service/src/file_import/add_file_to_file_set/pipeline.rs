use crate::{
    file_import::{
        add_file_to_file_set::{
            context::AddFileToFileSetContext,
            steps::{
                AddFileInfoToDatabaseStep, MarkFilesForCloudSyncStep, UpdateFileSetStep,
                ValidateFileStep,
            },
        },
        common_steps::{check_existing_files::CheckExistingFilesStep, import::ImportFilesStep},
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<AddFileToFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(ValidateFileStep),
            Box::new(CheckExistingFilesStep::<AddFileToFileSetContext>::new()),
            Box::new(ImportFilesStep::<AddFileToFileSetContext>::new()),
            Box::new(AddFileInfoToDatabaseStep),
            Box::new(UpdateFileSetStep),
            Box::new(MarkFilesForCloudSyncStep),
        ])
    }
}
