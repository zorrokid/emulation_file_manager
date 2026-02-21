use std::sync::{Arc, OnceLock};

use database::repository_manager::RepositoryManager;

use crate::{
    document_viewer_service::DocumentViewerService, download_service::DownloadService,
    emulator_service::EmulatorService, export_service::ExportService,
    external_executable_runner::service::ExternalExecutableRunnerService,
    file_import::service::FileImportService, file_set_deletion::service::FileSetDeletionService,
    file_set_download::service::DownloadService as FileSetDownloadService,
    mass_import::service::MassImportService, release_item_service::ReleaseItemService,
    release_service::ReleaseService, settings_service::SettingsService,
    software_title_service::SoftwareTitleService, system_service::SystemService,
    view_model_service::ViewModelService, view_models::Settings,
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
    file_import: OnceLock<Arc<FileImportService>>,
    download: OnceLock<Arc<DownloadService>>,
    export: OnceLock<Arc<ExportService>>,
    // TODO: combine with file set service
    file_set_deletion: OnceLock<Arc<FileSetDeletionService>>,
    file_set_download: OnceLock<Arc<FileSetDownloadService>>,
    runner: OnceLock<Arc<ExternalExecutableRunnerService>>,
    import: OnceLock<Arc<MassImportService>>,
    settings: OnceLock<Arc<SettingsService>>,
    repository_manager: Arc<RepositoryManager>,
    app_settings: Arc<Settings>,
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
            file_import: OnceLock::new(),
            download: OnceLock::new(),
            export: OnceLock::new(),
            file_set_deletion: OnceLock::new(),
            file_set_download: OnceLock::new(),
            runner: OnceLock::new(),
            import: OnceLock::new(),
            settings: OnceLock::new(),
            repository_manager,
            app_settings: settings,
        }
    }

    pub fn file_import(&self) -> Arc<FileImportService> {
        self.file_import
            .get_or_init(|| {
                Arc::new(FileImportService::new(
                    Arc::clone(&self.repository_manager),
                    Arc::clone(&self.app_settings),
                ))
            })
            .clone()
    }

    pub fn download(&self) -> Arc<DownloadService> {
        self.download
            .get_or_init(|| {
                Arc::new(DownloadService::new(
                    Arc::clone(&self.repository_manager),
                    Arc::clone(&self.app_settings),
                ))
            })
            .clone()
    }

    pub fn export(&self) -> Arc<ExportService> {
        self.export
            .get_or_init(|| Arc::new(ExportService::new(Arc::clone(&self.repository_manager))))
            .clone()
    }

    pub fn file_set_deletion(&self) -> Arc<FileSetDeletionService> {
        self.file_set_deletion
            .get_or_init(|| {
                Arc::new(FileSetDeletionService::new(
                    Arc::clone(&self.repository_manager),
                    Arc::clone(&self.app_settings),
                ))
            })
            .clone()
    }

    pub fn file_set_download(&self) -> Arc<FileSetDownloadService> {
        self.file_set_download
            .get_or_init(|| {
                Arc::new(FileSetDownloadService::new(
                    Arc::clone(&self.repository_manager),
                    Arc::clone(&self.app_settings),
                ))
            })
            .clone()
    }

    pub fn runner(&self) -> Arc<ExternalExecutableRunnerService> {
        self.runner
            .get_or_init(|| {
                Arc::new(ExternalExecutableRunnerService::new(
                    Arc::clone(&self.app_settings),
                    Arc::clone(&self.repository_manager),
                ))
            })
            .clone()
    }

    pub fn import(&self) -> Arc<MassImportService> {
        self.import
            .get_or_init(|| {
                Arc::new(MassImportService::new(
                    Arc::clone(&self.repository_manager),
                    Arc::clone(&self.app_settings),
                ))
            })
            .clone()
    }

    pub fn settings(&self) -> Arc<SettingsService> {
        self.settings
            .get_or_init(|| Arc::new(SettingsService::new(Arc::clone(&self.repository_manager))))
            .clone()
    }
}
