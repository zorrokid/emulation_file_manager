use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::repository::{
    emulator_repository::EmulatorRepository, file_info_repository::FileInfoRepository,
    file_set_repository::FileSetRepository, system_repository::SystemRepository,
};
pub struct RepositoryManager {
    file_info_repository: FileInfoRepository,
    file_set_repository: FileSetRepository,
    emulator_repository: EmulatorRepository,
    system_repository: SystemRepository,
}

impl RepositoryManager {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        let file_info_repository = FileInfoRepository::new(pool.clone());
        let file_set_repository = FileSetRepository::new(pool.clone());
        let emulator_repository = EmulatorRepository::new(pool.clone());
        let system_repository = SystemRepository::new(pool.clone());

        Self {
            file_info_repository,
            file_set_repository,
            emulator_repository,
            system_repository,
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
}
