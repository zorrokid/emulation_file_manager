use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use core_types::{FileType, Sha1Checksum};
use database::{models::System, repository_manager::RepositoryManager};
use file_export::{export_files_zipped, FileSetExportModel, OutputFile};

use crate::{
    error::Error,
    view_model_service::ViewModelService,
    view_models::{FileSetViewModel, Settings},
};

/// Service responsible for exporting all the files from the collection to a specified destination.
// TODO: refactor to use download service for exporting files
#[derive(Debug)]
pub struct ExportService {
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    settings: Arc<Settings>,
}

impl ExportService {
    pub fn new(
        repository_manager: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
        settings: Arc<Settings>,
    ) -> Self {
        Self {
            repository_manager,
            view_model_service,
            settings,
        }
    }

    fn construct_destination_path(
        &self,
        destination: &Path,
        systems: &[System],
        file_type: &FileType,
    ) -> PathBuf {
        let system_names: Vec<String> = systems.iter().map(|s| s.name.clone()).collect();
        let system_dir_name = system_names.join("_");
        let file_type_dir_name = file_type.dir_name();

        let mut path = PathBuf::from(destination);
        path.push(system_dir_name);
        path.push(file_type_dir_name);
        path
    }

    // TODO: use download service to export all files
    #[deprecated]
    pub async fn export_all_files(&self, destination: &Path) -> Result<(), Error> {
        println!("Exporting all files to {}", destination.display());

        let file_sets = self
            .repository_manager
            .get_file_set_repository()
            .get_all_file_sets()
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        for file_set in file_sets {
            println!("Processing file set: {}", file_set.id);
            let file_set_view_model = self
                .view_model_service
                .get_file_set_view_model(file_set.id)
                .await?;

            println!("Files in file set: {:?}", file_set_view_model.files);

            let systems = self
                .repository_manager
                .get_system_repository()
                .get_systems_by_file_set(file_set.id)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;

            println!("Systems for file set: {:?}", systems);

            let destination_path = self.construct_destination_path(
                destination,
                &systems,
                &file_set_view_model.file_type,
            );

            std::fs::create_dir_all(&destination_path)
                .map_err(|e| Error::IoError(e.to_string()))?;

            println!("Destionation path: {:?}", destination_path.display());

            let export_model = prepare_fileset_for_export(
                &file_set_view_model,
                &self.settings.collection_root_dir,
                &destination_path,
                true,
            );

            println!("Export model: {:?}", export_model);

            export_files_zipped(&export_model).map_err(|e| Error::ExportError(e.to_string()))?;
        }

        Ok(())
    }
}

// TODO use get_file_type_path from Settings, this is deprecated
#[deprecated]
pub fn resolve_file_type_path(root_path: &Path, file_type: &core_types::FileType) -> PathBuf {
    let mut path = PathBuf::from(root_path);
    path.push(file_type.dir_name());
    path
}

// TODO: this will be replaced by dowload service
#[deprecated]
pub fn prepare_fileset_for_export(
    file_set: &FileSetViewModel,
    collection_root_dir: &Path,
    output_dir: &Path, // TODO: remove? this is not necessary here
    extract_files: bool,
) -> FileSetExportModel {
    let source_file_path = resolve_file_type_path(collection_root_dir, &file_set.file_type);

    let output_mapping = file_set
        .files
        .iter()
        .map(|f| {
            let checksum: Sha1Checksum = f
                .sha1_checksum
                .clone()
                .try_into()
                .expect("Failed to convert to Sha1Checksum");
            (
                f.archive_file_name.clone(),
                OutputFile {
                    output_file_name: f.file_name.clone(),
                    checksum,
                },
            )
        })
        .collect::<HashMap<String, OutputFile>>();

    let exported_zip_file_name = file_set.file_set_name.clone();

    FileSetExportModel {
        output_mapping,
        source_file_path,
        output_dir: output_dir.to_path_buf(),
        extract_files,
        exported_zip_file_name,
    }
}
