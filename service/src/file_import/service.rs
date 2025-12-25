use std::{path::Path, sync::Arc};

use core_types::FileType;
use database::repository_manager::RepositoryManager;
use file_import::FileImportOps;

use crate::{
    error::Error,
    file_import::{
        add_file_to_file_set::context::AddFileToFileSetContext,
        import::context::FileImportContext,
        model::{
            AddToFileSetImportModel, FileImportData, FileImportPrepareResult, FileSetImportModel,
        },
        prepare::context::PrepareFileImportContext,
    },
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

pub struct FileImportService {
    repository_manager: Arc<RepositoryManager>,
    fs_ops: Arc<dyn FileSystemOps>,
    file_import_ops: Arc<dyn FileImportOps>,
    settings: Arc<Settings>,
}

impl std::fmt::Debug for FileImportService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileImportService").finish_non_exhaustive()
    }
}

impl FileImportService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self::new_with_ops(
            repository_manager,
            Arc::new(StdFileSystemOps),
            Arc::new(file_import::StdFileImportOps),
            settings,
        )
    }

    pub fn new_with_ops(
        repository_manager: Arc<RepositoryManager>,
        fs_ops: Arc<dyn FileSystemOps>,
        file_import_ops: Arc<dyn FileImportOps>,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            repository_manager,
            fs_ops,
            file_import_ops,
            settings,
        }
    }

    pub async fn prepare_import(
        &self,
        file_path: &Path,
        file_type: FileType,
    ) -> Result<FileImportPrepareResult, Error> {
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
                let import_model = context.get_imported_file_info();
                let import_metadata = context.import_metadata.ok_or_else(|| {
                    Error::FileImportError("Import metadata not set after preparation".to_string())
                })?;
                Ok(FileImportPrepareResult {
                    import_model,
                    import_metadata,
                })
            }
            Err(err) => {
                tracing::error!(error = %err, "Failed to prepare file import");
                Err(err)
            }
        }
    }

    pub async fn import(&self, import_model: FileSetImportModel) -> Result<i64, Error> {
        let file_type = import_model.file_type;
        let output_dir = self.settings.collection_root_dir.clone();
        let file_import_data = FileImportData {
            output_dir,
            file_type,
            selected_files: import_model.selected_files,
            import_files: import_model.import_files,
        };

        let mut context = FileImportContext {
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
            file_import_data,
            system_ids: import_model.system_ids,
            source: import_model.source,
            file_set_name: import_model.file_set_name,
            file_set_file_name: import_model.file_set_file_name,
            imported_files: std::collections::HashMap::new(),
            file_set_id: None,
            file_import_ops: self.file_import_ops.clone(),
            file_system_ops: self.fs_ops.clone(),
            existing_files: vec![],
        };
        let pipeline = Pipeline::<FileImportContext>::new();
        let result = pipeline.execute(&mut context).await;
        match (result, context.file_set_id) {
            (Ok(_), Some(id)) => Ok(id),
            (Err(err), _) => Err(err),
            (_, None) => Err(Error::FileImportError(
                "File set ID not set after import".to_string(),
            )),
        }
    }

    pub async fn import_and_add_files_to_file_set(
        &self,
        import_model: AddToFileSetImportModel,
    ) -> Result<(), Error> {
        let file_import_data = FileImportData {
            output_dir: self.settings.collection_root_dir.clone(),
            file_type: import_model.file_type, // TODO make this optional? this is required only
            // when adding new file set
            selected_files: import_model.selected_files,
            import_files: import_model.import_files,
        };
        let mut context = AddFileToFileSetContext::new(
            self.repository_manager.clone(),
            self.settings.clone(),
            self.file_import_ops.clone(),
            self.fs_ops.clone(),
            import_model.file_set_id,
            file_import_data,
        );
        let pipeline = Pipeline::<AddFileToFileSetContext>::new();
        pipeline.execute(&mut context).await
    }
}
