use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use core_types::{FileType, ImportedFile, Sha1Checksum};
use database::repository_manager::RepositoryManager;
use file_import::{FileImportModel, FileImportOps};

use crate::{file_import::model::FileImportModel as ServiceFileImportModel, view_models::Settings};

pub struct FileImportContext {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub selected_files: Vec<Sha1Checksum>,
    pub file_type: FileType,
    // It is possible to create a file set from multiple sets of import files.
    // From this collection of files, the ones that are in selected_files and are new files, will be imported.
    // If the files are existing files, they will be just linked to the file set.
    // When files are imported, their archive file names will be populated here.
    pub import_files: Vec<ServiceFileImportModel>,
    pub system_ids: Vec<i64>,
    pub source: String,
    // File set nama and file name the will be created from the set of import files.
    pub file_set_name: String,
    pub file_set_file_name: String,
    pub imported_files: HashMap<Sha1Checksum, ImportedFile>,
    pub file_set_id: Option<i64>,
    pub file_import_ops: Arc<dyn FileImportOps>,
}

impl FileImportContext {
    pub fn get_new_selected_file_names(&self) -> HashSet<String> {
        self.import_files
            .iter()
            .flat_map(|file| {
                file.content
                    .iter()
                    .filter_map(|(sha1_checksum, import_content)| {
                        if self.selected_files.contains(sha1_checksum)
                            && import_content.existing_file_info_id.is_none()
                        {
                            Some(import_content.file_name.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect::<HashSet<String>>()
    }

    pub fn get_files_in_file_set(&self) -> Vec<ImportedFile> {
        let mut files_in_file_set: Vec<ImportedFile> =
            self.imported_files.values().cloned().collect();

        // add existing files that were selected
        self.import_files.iter().for_each(|file| {
            file.content
                .iter()
                .for_each(|(sha1_checksum, file_content)| {
                    if self.selected_files.contains(sha1_checksum)
                        && let Some(existing_achive_file_name) =
                            &file_content.existing_archive_file_name
                    {
                        files_in_file_set.push(ImportedFile {
                            original_file_name: file_content.file_name.clone(),
                            sha1_checksum: *sha1_checksum,
                            file_size: file_content.file_size,
                            archive_file_name: existing_achive_file_name.clone(),
                        });
                    }
                });
        });

        files_in_file_set
    }

    pub fn get_file_import_model(&self) -> FileImportModel {
        let target_path = self.settings.get_file_type_path(&self.file_type);
        FileImportModel {
            file_path: self
                .import_files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<PathBuf>>(),
            output_dir: target_path.to_path_buf(),
            file_name: self.file_set_file_name.clone(),
            file_set_name: self.file_set_name.clone(),
            file_type: self.file_type,
            new_files_file_name_filter: self.get_new_selected_file_names(),
        }
    }
}
