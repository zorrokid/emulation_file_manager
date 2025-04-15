use std::sync::Arc;

use database::{database_error::DatabaseError, repository_manager::RepositoryManager};

use crate::view_models::EmulatorViewModel;

pub struct ViewModelService {
    repository_manager: Arc<RepositoryManager>,
}

impl ViewModelService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }
}
