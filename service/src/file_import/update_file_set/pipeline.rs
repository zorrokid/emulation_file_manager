use crate::{
    file_import::{
        common_steps::{
            check_existing_files::CheckExistingFilesStep,
            file_deletion_steps::{
                DeleteFileInfosStep, DeleteLocalFilesStep, FilterDeletableFilesStep,
                MarkForCloudDeletionStep,
            },
            import::ImportFilesStep,
        },
        update_file_set::{
            context::UpdateFileSetContext,
            steps::{
                CollectDeletionCandidatesStep, FetchFileSetStep, FetchFilesInFileSetStep,
                MarkFilesForCloudSyncStep, UnlinkFilesFromFileSetStep,
                UpdateFileInfoToDatabaseStep, UpdateFileSetFilesStep,
            },
        },
    },
    pipeline::generic_pipeline::Pipeline,
};

impl Pipeline<UpdateFileSetContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            // preparations steps
            Box::new(FetchFileSetStep),
            Box::new(FetchFilesInFileSetStep),
            // file deletion steps
            Box::new(FilterDeletableFilesStep::<UpdateFileSetContext>::new()),
            Box::new(DeleteLocalFilesStep::<UpdateFileSetContext>::new()),
            Box::new(MarkForCloudDeletionStep::<UpdateFileSetContext>::new()),
            Box::new(UnlinkFilesFromFileSetStep),
            Box::new(DeleteFileInfosStep::<UpdateFileSetContext>::new()),
            // import new files
            Box::new(CheckExistingFilesStep::<UpdateFileSetContext>::new()),
            Box::new(ImportFilesStep::<UpdateFileSetContext>::new()),
            Box::new(UpdateFileInfoToDatabaseStep),
            Box::new(CollectDeletionCandidatesStep),
            Box::new(UpdateFileSetFilesStep),
            Box::new(MarkFilesForCloudSyncStep),
        ])
    }
}
