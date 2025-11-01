use crate::{
    cloud_sync::{
        context::SyncContext,
        steps::{
            ConnectToCloudStep, DeleteMarkedFilesStep, GetSyncFileCountsStep,
            PrepareFilesForUploadStep, UploadPendingFilesStep,
        },
    },
    pipeline::Pipeline,
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
            Box::new(ConnectToCloudStep),
            Box::new(UploadPendingFilesStep),
            Box::new(DeleteMarkedFilesStep),
        ])
    }
}
