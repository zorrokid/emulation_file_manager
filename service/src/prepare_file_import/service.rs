use std::{path::Path, sync::Arc};

use core_types::FileType;
use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    pipeline::generic_pipeline::Pipeline,
    prepare_file_import::context::{ImportFile, PrepareFileImportContext},
};

pub struct PrepareFileImportService {
    repository_manager: Arc<RepositoryManager>,
}

impl PrepareFileImportService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn prepare_import(
        &self,
        file_path: &Path,
        file_type: FileType,
    ) -> Result<ImportFile, Error> {
        let mut context =
            PrepareFileImportContext::new(self.repository_manager.clone(), file_path, file_type);
        let pipeline = Pipeline::<PrepareFileImportContext>::new();
        match pipeline.execute(&mut context).await {
            Ok(_) => {
                let import_file_info = context.get_imported_file_info();
                Ok(import_file_info)
            }
            Err(err) => {
                tracing::error!(error = %err, "Failed to prepare file import");
                Err(err)
            }
        }
    }
}
