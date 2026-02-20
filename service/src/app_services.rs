use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::{
    document_viewer_service::DocumentViewerService,
    emulator_service::EmulatorService,
    release_item_service::ReleaseItemService,
    release_service::ReleaseService,
    software_title_service::SoftwareTitleService,
    system_service::SystemService,
    view_model_service::ViewModelService,
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
}

impl AppServices {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self {
            view_model: Arc::new(ViewModelService::new(Arc::clone(&repository_manager))),
            system: Arc::new(SystemService::new(Arc::clone(&repository_manager))),
            release: Arc::new(ReleaseService::new(Arc::clone(&repository_manager))),
            release_item: Arc::new(ReleaseItemService::new(Arc::clone(&repository_manager))),
            software_title: Arc::new(SoftwareTitleService::new(Arc::clone(&repository_manager))),
            emulator: Arc::new(EmulatorService::new(Arc::clone(&repository_manager))),
            document_viewer: Arc::new(DocumentViewerService::new(Arc::clone(&repository_manager))),
        }
    }
}
