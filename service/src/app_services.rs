use std::sync::{Arc, OnceLock};

use database::{get_db_pool, repository_manager::RepositoryManager};

use crate::{
    cloud_sync::service::CloudStorageSyncService, document_viewer_service::DocumentViewerService,
    download_service::DownloadService, emulator_service::EmulatorService,
    export_service::ExportService,
    external_executable_runner::service::ExternalExecutableRunnerService,
    file_import::service::FileImportService, file_set_deletion::service::FileSetDeletionService,
    file_set_download::service::DownloadService as FileSetDownloadService,
    mass_import::service::MassImportService, release_item_service::ReleaseItemService,
    release_service::ReleaseService, settings_service::SettingsService,
    software_title_service::SoftwareTitleService, system_service::SystemService,
    view_model_service::ViewModelService, view_models::Settings,
};

pub async fn create_app_services() -> Arc<AppServices> {
    let pool = get_db_pool().await.expect("DB pool initialization failed");
    let repository_manager = Arc::new(RepositoryManager::new(pool));

    let settings: Settings = repository_manager
        .get_settings_repository()
        .get_settings()
        .await
        .expect("Failed to get settings from repository")
        .into();

    let settings = Arc::new(settings);

    Arc::new(AppServices::new(
        Arc::clone(&repository_manager),
        Arc::clone(&settings),
    ))
}

#[derive(Debug)]
pub struct AppServices {
    view_model: OnceLock<Arc<ViewModelService>>,
    system: OnceLock<Arc<SystemService>>,
    release: OnceLock<Arc<ReleaseService>>,
    release_item: OnceLock<Arc<ReleaseItemService>>,
    software_title: OnceLock<Arc<SoftwareTitleService>>,
    emulator: OnceLock<Arc<EmulatorService>>,
    document_viewer: OnceLock<Arc<DocumentViewerService>>,
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
    cloud_storage: OnceLock<Arc<CloudStorageSyncService>>,
}

impl AppServices {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self {
            view_model: OnceLock::new(),
            system: OnceLock::new(),
            release: OnceLock::new(),
            release_item: OnceLock::new(),
            software_title: OnceLock::new(),
            emulator: OnceLock::new(),
            document_viewer: OnceLock::new(),
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
            cloud_storage: OnceLock::new(),
        }
    }

    pub fn view_model(&self) -> Arc<ViewModelService> {
        self.view_model
            .get_or_init(|| Arc::new(ViewModelService::new(Arc::clone(&self.repository_manager))))
            .clone()
    }

    pub fn system(&self) -> Arc<SystemService> {
        self.system
            .get_or_init(|| Arc::new(SystemService::new(Arc::clone(&self.repository_manager))))
            .clone()
    }

    pub fn release(&self) -> Arc<ReleaseService> {
        self.release
            .get_or_init(|| Arc::new(ReleaseService::new(Arc::clone(&self.repository_manager))))
            .clone()
    }

    pub fn release_item(&self) -> Arc<ReleaseItemService> {
        self.release_item
            .get_or_init(|| {
                Arc::new(ReleaseItemService::new(Arc::clone(
                    &self.repository_manager,
                )))
            })
            .clone()
    }
    pub fn software_title(&self) -> Arc<SoftwareTitleService> {
        self.software_title
            .get_or_init(|| {
                Arc::new(SoftwareTitleService::new(Arc::clone(
                    &self.repository_manager,
                )))
            })
            .clone()
    }
    pub fn emulator(&self) -> Arc<EmulatorService> {
        self.emulator
            .get_or_init(|| Arc::new(EmulatorService::new(Arc::clone(&self.repository_manager))))
            .clone()
    }

    pub fn document_viewer(&self) -> Arc<DocumentViewerService> {
        self.document_viewer
            .get_or_init(|| {
                Arc::new(DocumentViewerService::new(Arc::clone(
                    &self.repository_manager,
                )))
            })
            .clone()
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

    pub fn cloud_storage(&self) -> Arc<CloudStorageSyncService> {
        self.cloud_storage
            .get_or_init(|| {
                Arc::new(CloudStorageSyncService::new(
                    Arc::clone(&self.repository_manager),
                    Arc::clone(&self.app_settings),
                ))
            })
            .clone()
    }

    pub fn app_settings(&self) -> Arc<Settings> {
        Arc::clone(&self.app_settings)
    }
}
