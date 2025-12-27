use crate::{
    file_set_download::{
        context::DownloadContext,
        steps::{
            DownloadFilesStep, ExportFilesStep, FetchFileSetFileInfoStep, FetchFileSetStep,
            PrepareFileForDownloadStep, PrepareThumbnailsStep,
        },
    },
    pipeline::{
        cloud_connection::ConnectToCloudStep, generic_pipeline::Pipeline,
        test_cloud_connection::TestConnectToCloudStep,
    },
};

impl Pipeline<DownloadContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(FetchFileSetStep),
            Box::new(FetchFileSetFileInfoStep),
            Box::new(PrepareFileForDownloadStep),
            Box::new(ConnectToCloudStep::<DownloadContext>::new()),
            Box::new(TestConnectToCloudStep::<DownloadContext>::new()),
            Box::new(DownloadFilesStep),
            Box::new(ExportFilesStep),
            Box::new(PrepareThumbnailsStep),
        ])
    }
}
