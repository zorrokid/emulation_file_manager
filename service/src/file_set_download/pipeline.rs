use crate::{
    cloud_connection::ConnectToCloudStep,
    file_set_download::{
        context::DownloadContext,
        steps::{
            DownloadFilesStep, ExportFilesStep, FetchFileSetStep,
            PrepareFileForDownloadStep, FetchFileSetFileInfoStep,
        },
    },
    pipeline::Pipeline,
};

impl Pipeline<DownloadContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(FetchFileSetStep),
            Box::new(FetchFileSetFileInfoStep),
            Box::new(PrepareFileForDownloadStep),
            Box::new(ConnectToCloudStep::<DownloadContext>::new()),
            Box::new(DownloadFilesStep),
            Box::new(ExportFilesStep),
        ])
    }
}
