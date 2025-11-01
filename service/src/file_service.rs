use std::sync::Arc;

use cloud_storage::CloudStorageOps;

use crate::view_models::{FileSetViewModel, Settings};

/// all files are accessed through this service
pub struct FileService {
    settings: Arc<Settings>,
    cloud_ops: Option<Arc<dyn CloudStorageOps>>,
}

/*impl FileService {
    /// Create a new instance of FileService
    pub fn new(settings: Arc<Settings>) -> Self {
        Self { settings }
    }

    /// Prepare the file set for use by ensuring all files are available locally.
    /// If files are not available locally, they will be downloaded from cloud storage to
    /// collection directory, and then exported to the configured temp directory.
    /// # Arguments
    /// * `file_set` - The file set view model containing information about the file set, including
    ///   files in the set.
    /// * `extract_files` - A boolean indicating whether to extract files during export.
    ///
    pub fn prepare_file_set_for_use(&self, file_set: FileSetViewModel, extract_files: bool) {
        // go throught the files in files set and check which are missing locally
        let collection_root = &self.settings.collection_root_dir;

        let mut files_to_download = vec![];

        for file in &file_set.files {
            let file_path = self
                .settings
                .get_file_path(&file_set.file_type, &file.archive_file_name);

            if !file_path.exists() {
                files_to_download.push(file.clone());
            }
        }

        for file in files_to_download {
            download_file(
        }

        let export_model = prepare_fileset_for_export(
            &file_set,
            &self.settings.collection_root_dir,
            &self.settings.temp_output_dir,
            extract_files,
        );

        // check if files are locally available
        // if not, download from cloud storage to collection directory
        // then export as usual
    }
}*/
