use std::path::Path;
use std::sync::Arc;

use core_types::FileType;
use database::repository_manager::RepositoryManager;

use crate::error::Error;
use crate::file_import::model::FileImportPrepareResult;
use crate::file_import::service::FileImportService;
use crate::view_models::Settings;

#[derive(Debug)]
pub struct DownloadService {
    file_import_service: Arc<FileImportService>,
}

impl DownloadService {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        settings: Arc<Settings>,
    ) -> Self {
        let file_import_service = Arc::new(FileImportService::new(
            repository_manager,
            settings,
        ));
        
        Self {
            file_import_service,
        }
    }

    /// Download a file from URL and prepare it for import
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to download from
    /// * `file_type` - The type of file being downloaded
    /// * `temp_dir` - Temporary directory to download the file to
    ///
    /// # Returns
    ///
    /// Result containing FileImportPrepareResult which can be used
    /// with the existing file import UI
    pub async fn download_and_prepare_import(
        &self,
        url: &str,
        file_type: FileType,
        temp_dir: &Path,
    ) -> Result<FileImportPrepareResult, Error> {
        let download_result = http_downloader::download_file(url, temp_dir)
            .await
            .map_err(|e| Error::DownloadError(e.to_string()))?;

        let prepare_result = self.file_import_service
            .prepare_import(&download_result.file_path, file_type)
            .await?;

        Ok(prepare_result)
    }
}
