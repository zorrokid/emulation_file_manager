use std::{path::Path, sync::Arc};

use core_types::FileType;
use database::repository_manager::RepositoryManager;
use file_import::FileImportOps;

use crate::{
    error::Error,
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    prepare_file_import::{context::PrepareFileImportContext, model::ImportFile},
};

pub struct PrepareFileImportService {
    repository_manager: Arc<RepositoryManager>,
    fs_ops: Arc<dyn FileSystemOps>,
    file_import_ops: Arc<dyn FileImportOps>,
}

impl std::fmt::Debug for PrepareFileImportService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PrepareFileImportService")
            .finish_non_exhaustive()
    }
}

impl PrepareFileImportService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self::new_with_ops(
            repository_manager,
            Arc::new(StdFileSystemOps),
            Arc::new(file_import::StdFileImportOps),
        )
    }

    pub fn new_with_ops(
        repository_manager: Arc<RepositoryManager>,
        fs_ops: Arc<dyn FileSystemOps>,
        file_import_ops: Arc<dyn FileImportOps>,
    ) -> Self {
        Self {
            repository_manager,
            fs_ops,
            file_import_ops,
        }
    }

    pub async fn prepare_import(
        &self,
        file_path: &Path,
        file_type: FileType,
    ) -> Result<ImportFile, Error> {
        let mut context = PrepareFileImportContext::new(
            self.repository_manager.clone(),
            file_path,
            file_type,
            self.fs_ops.clone(),
            self.file_import_ops.clone(),
        );
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
