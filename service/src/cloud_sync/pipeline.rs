use crate::{
    cloud_sync::{
        context::SyncContext,
        steps::{
            CleanupOrphanedSyncLogsStep, DeleteMarkedFilesStep, GetSyncFileCountsStep,
            PrepareFilesForUploadStep, UploadPendingFilesStep,
        },
    },
    pipeline::{cloud_connection::ConnectToCloudStep, generic_pipeline::Pipeline},
};

impl Default for Pipeline<SyncContext> {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline<SyncContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(PrepareFilesForUploadStep),
            Box::new(GetSyncFileCountsStep),
            Box::new(ConnectToCloudStep::<SyncContext>::new()),
            Box::new(UploadPendingFilesStep),
            Box::new(DeleteMarkedFilesStep),
            Box::new(CleanupOrphanedSyncLogsStep),
        ])
    }
}
