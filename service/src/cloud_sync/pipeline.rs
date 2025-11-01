use crate::{
    cloud_connection::ConnectToCloudStep,
    cloud_sync::{
        context::SyncContext,
        steps::{
            DeleteMarkedFilesStep, GetSyncFileCountsStep,
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
            Box::new(ConnectToCloudStep::<SyncContext>::new()),
            Box::new(UploadPendingFilesStep),
            Box::new(DeleteMarkedFilesStep),
        ])
    }
}
