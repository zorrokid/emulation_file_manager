use std::{collections::HashMap, path::Path};

use core_types::{FileType, ImportedFile};
use utils::test_utils::generate_random_uuid;

use crate::{
    error::Error,
    file_import::{
        model::{
            FileImportPrepareResult, FileImportResult, FileSetImportModel, UpdateFileSetModel,
        },
        service::FileImportService,
    },
};

#[async_trait::async_trait]
pub trait FileImportServiceOps: Send + Sync {
    async fn prepare_import(
        &self,
        file_path: &Path,
        file_type: FileType,
    ) -> Result<FileImportPrepareResult, Error>;

    async fn create_file_set(
        &self,
        import_model: FileSetImportModel,
    ) -> Result<FileImportResult, Error>;

    async fn update_file_set(
        &self,
        import_model: UpdateFileSetModel,
    ) -> Result<FileImportResult, Error>;
}

#[async_trait::async_trait]
impl FileImportServiceOps for FileImportService {
    async fn prepare_import(
        &self,
        file_path: &Path,
        file_type: FileType,
    ) -> Result<FileImportPrepareResult, Error> {
        self.prepare_import(file_path, file_type).await
    }

    async fn create_file_set(
        &self,
        import_model: FileSetImportModel,
    ) -> Result<FileImportResult, Error> {
        self.create_file_set(import_model).await
    }

    async fn update_file_set(
        &self,
        import_model: UpdateFileSetModel,
    ) -> Result<FileImportResult, Error> {
        self.update_file_set(import_model).await
    }
}

pub struct CreateMockState {
    pub file_set_id: i64,
    pub release_id: Option<i64>,
}

#[derive(Default)]
pub struct MockFileImportServiceOps {
    pub should_fail: bool,
    pub create_calls: Vec<FileSetImportModel>,
    pub update_calls: Vec<UpdateFileSetModel>,
    pub prepare_calls: Vec<(String, FileType)>,
    pub setup_create_mock: Option<CreateMockState>,
}

impl MockFileImportServiceOps {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn with_create_mock(setup: CreateMockState) -> Self {
        Self {
            setup_create_mock: Some(setup),
            ..Default::default()
        }
    }
}

#[async_trait::async_trait]
impl FileImportServiceOps for MockFileImportServiceOps {
    async fn prepare_import(
        &self,
        _file_path: &Path,

        _file_type: FileType,
    ) -> Result<FileImportPrepareResult, Error> {
        unimplemented!()
    }

    /// Simulates the creation of a file set based on the provided import model.
    async fn create_file_set(
        &self,
        import_model: FileSetImportModel,
    ) -> Result<FileImportResult, Error> {
        if self.should_fail {
            return Err(Error::FileImportError(
                "Mock create_file_set failure".to_string(),
            ));
        }
        match &self.setup_create_mock {
            Some(setup) => {
                // TODO: simulates that all files are new, implement other scenarios as needed
                let imported_new_files: Vec<ImportedFile> = import_model
                    .import_files
                    .iter()
                    .flat_map(|f| {
                        f.content.values().map(|c| ImportedFile {
                            original_file_name: c.file_name.clone(),
                            archive_file_name: generate_random_uuid(),
                            sha1_checksum: c.sha1_checksum.clone(),
                            file_size: c.file_size,
                        })
                    })
                    .collect();
                let file_import_result = FileImportResult {
                    file_set_id: setup.file_set_id,
                    release_id: setup.release_id,
                    imported_new_files,
                    // TODO: implement failed files as needed
                    failed_steps: HashMap::new(),
                };
                Ok(file_import_result)
            }
            None => Err(Error::FileImportError(
                "No mock setup for create_file_set".to_string(),
            )),
        }
    }

    async fn update_file_set(
        &self,
        _import_model: UpdateFileSetModel,
    ) -> Result<FileImportResult, Error> {
        unimplemented!()
    }
}
