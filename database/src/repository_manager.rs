use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::repository::{
    file_info_repository::FileInfoRepository, file_set_repository::FileSetRepository,
};
pub struct RepositoryManager {
    pool: Arc<Pool<Sqlite>>,
    file_info_repository: FileInfoRepository,
    file_set_repository: FileSetRepository,
}

impl RepositoryManager {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        let file_info_repository = FileInfoRepository::new(pool.clone());
        let file_set_repository = FileSetRepository::new(pool.clone());
        Self {
            pool,
            file_info_repository,
            file_set_repository,
        }
    }

    pub fn get_file_info_repository(&self) -> &FileInfoRepository {
        &self.file_info_repository
    }

    pub fn get_file_set_repository(&self) -> &FileSetRepository {
        &self.file_set_repository
    }
}
