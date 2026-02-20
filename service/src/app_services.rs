use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::{
    document_viewer_service::DocumentViewerService, download_service::DownloadService,
    emulator_service::EmulatorService, export_service::ExportService,
    file_import::service::FileImportService, release_item_service::ReleaseItemService,
    release_service::ReleaseService, software_title_service::SoftwareTitleService,
    system_service::SystemService, view_model_service::ViewModelService, view_models::Settings,
};

#[derive(Debug)]
pub struct AppServices {
    pub view_model: Arc<ViewModelService>,
    pub system: Arc<SystemService>,
    pub release: Arc<ReleaseService>,
    pub release_item: Arc<ReleaseItemService>,
    pub software_title: Arc<SoftwareTitleService>,
    pub emulator: Arc<EmulatorService>,
    pub document_viewer: Arc<DocumentViewerService>,
    pub file_import: Arc<FileImportService>,
    pub download: Arc<DownloadService>,
    pub export: Arc<ExportService>,
}

impl AppServices {
    // TODO: maybe just store the repository manager and settings in AppServices and create the
    // individual services on demand?
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self {
            view_model: Arc::new(ViewModelService::new(Arc::clone(&repository_manager))),
            system: Arc::new(SystemService::new(Arc::clone(&repository_manager))),
            release: Arc::new(ReleaseService::new(Arc::clone(&repository_manager))),
            release_item: Arc::new(ReleaseItemService::new(Arc::clone(&repository_manager))),
            software_title: Arc::new(SoftwareTitleService::new(Arc::clone(&repository_manager))),
            emulator: Arc::new(EmulatorService::new(Arc::clone(&repository_manager))),
            document_viewer: Arc::new(DocumentViewerService::new(Arc::clone(&repository_manager))),
            file_import: Arc::new(FileImportService::new(
                Arc::clone(&repository_manager),
                Arc::clone(&settings),
            )),
            download: Arc::new(DownloadService::new(
                Arc::clone(&repository_manager),
                Arc::clone(&settings),
            )),
            export: Arc::new(ExportService::new(Arc::clone(&repository_manager))),
        }
    }
}
