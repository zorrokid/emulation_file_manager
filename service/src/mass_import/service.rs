use std::{path::PathBuf, sync::Arc};

use core_types::FileType;
use database::repository_manager::RepositoryManager;

use crate::error::Error;

#[derive(Debug)]
pub struct MassImportService {
    repository_manager: Arc<RepositoryManager>,
}

impl MassImportService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        MassImportService { repository_manager }
    }

    pub async fn import(
        &self,
        system_id: i64,
        source_path: PathBuf,
        dat_file_path: Option<PathBuf>,
        file_type: FileType,
    ) -> Result<(), Error> {
        tracing::info!(
            system_id = system_id,
            source_path = ?source_path,
            dat_file_path = ?dat_file_path,
            file_type = ?file_type,
            "Starting mass import process...");
        // Implementation of mass import logic goes here
        Ok(())
    }
}
