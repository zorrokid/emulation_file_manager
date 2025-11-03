use crate::{
    file_set_download::{
        context::DownloadContext,
        steps::{
            DownloadFilesStep, ExportFilesStep, FetchFileSetFileInfoStep, FetchFileSetStep,
            PrepareFileForDownloadStep,
        },
    },
    file_system_ops::FileSystemOps,
    pipeline::{cloud_connection::ConnectToCloudStep, generic_pipeline::Pipeline},
};

impl<F: FileSystemOps + 'static> Pipeline<DownloadContext<F>> {
    pub fn new() -> Self {
        Self::with_steps(vec![
            Box::new(FetchFileSetStep),
            Box::new(FetchFileSetFileInfoStep),
            Box::new(PrepareFileForDownloadStep),
            Box::new(ConnectToCloudStep::<DownloadContext<F>>::new()),
            Box::new(DownloadFilesStep),
            Box::new(ExportFilesStep),
        ])
    }
}
