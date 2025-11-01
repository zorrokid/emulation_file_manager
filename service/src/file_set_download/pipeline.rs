use crate::{
    file_set_download::{
        context::DownloadContext,
        steps::{
            ConnectToCloudStep, DownloadFilesStep, ExportFilesStep, FetchFileInfoStep,
            PrepareFileForDownloadStep,
        },
    },
    pipeline::Pipeline,
};

impl Pipeline<DownloadContext> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(FetchFileInfoStep),
            Box::new(PrepareFileForDownloadStep),
            Box::new(ConnectToCloudStep),
            Box::new(DownloadFilesStep),
            Box::new(ExportFilesStep),
        ])
    }
}
