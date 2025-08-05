use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::repository::{
    document_viewer_repository::DocumentViewerRepository, emulator_repository::EmulatorRepository,
    file_info_repository::FileInfoRepository, file_set_repository::FileSetRepository,
    franchise_repository::FranchiseRepository, release_repository::ReleaseRepository,
    setting_repository::SettingRepository, software_title_repository::SoftwareTitleRepository,
    system_repository::SystemRepository,
};

#[derive(Debug)]
pub struct RepositoryManager {
    file_info_repository: FileInfoRepository,
    file_set_repository: FileSetRepository,
    emulator_repository: EmulatorRepository,
    system_repository: SystemRepository,
    franchise_repository: FranchiseRepository,
    release_repository: ReleaseRepository,
    software_title_repository: SoftwareTitleRepository,
    setting_repository: SettingRepository,
    document_viewer_repository: DocumentViewerRepository,
}

impl RepositoryManager {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        let file_info_repository = FileInfoRepository::new(pool.clone());
        let file_set_repository = FileSetRepository::new(pool.clone());
        let emulator_repository = EmulatorRepository::new(pool.clone());
        let system_repository = SystemRepository::new(pool.clone());
        let franchise_repository = FranchiseRepository::new(pool.clone());
        let release_repository = ReleaseRepository::new(pool.clone());
        let software_title_repository = SoftwareTitleRepository::new(pool.clone());
        let setting_repository = SettingRepository::new(pool.clone());
        let document_viewer_repository = DocumentViewerRepository::new(pool.clone());

        Self {
            file_info_repository,
            file_set_repository,
            emulator_repository,
            system_repository,
            franchise_repository,
            release_repository,
            software_title_repository,
            setting_repository,
            document_viewer_repository,
        }
    }

    pub fn get_file_info_repository(&self) -> &FileInfoRepository {
        &self.file_info_repository
    }

    pub fn get_file_set_repository(&self) -> &FileSetRepository {
        &self.file_set_repository
    }

    pub fn get_emulator_repository(&self) -> &EmulatorRepository {
        &self.emulator_repository
    }

    pub fn get_system_repository(&self) -> &SystemRepository {
        &self.system_repository
    }

    pub fn get_franchise_repository(&self) -> &FranchiseRepository {
        &self.franchise_repository
    }

    pub fn get_release_repository(&self) -> &ReleaseRepository {
        &self.release_repository
    }

    pub fn get_software_title_repository(&self) -> &SoftwareTitleRepository {
        &self.software_title_repository
    }

    pub fn settings(&self) -> &SettingRepository {
        &self.setting_repository
    }

    pub fn get_document_viewer_repository(&self) -> &DocumentViewerRepository {
        &self.document_viewer_repository
    }
}
