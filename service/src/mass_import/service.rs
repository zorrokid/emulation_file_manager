use std::{path::PathBuf, sync::Arc};

use core_types::{FileType, item_type::ItemType};
use database::repository_manager::RepositoryManager;

use crate::{
    error::Error, mass_import::context::MassImportContext, pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

#[derive(Debug)]
pub struct MassImportService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
}

impl MassImportService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        MassImportService {
            repository_manager,
            settings,
        }
    }

    /// Starts the mass import process for the given system ID and source path.
    /// For each file or archive found in the source path, it will attempt to read metadata,
    /// match against the DAT file (if provided), and import the files into the collection and
    /// database. It will create a file set for each file or archive successfully imported and a
    /// release with software title linked to the file sets.
    ///
    /// TODO: should we try to use existing software titles and releases if they already exist?
    ///
    /// For simplicity, let's start with creating new software titles and releases for each import.
    ///
    /// User can remove duplicated from UI. Theere will be also a functionality to merge software
    /// titles and releases in the future.
    /// - when merging two software titles, all linked releases will be moved to the target
    /// software title.
    /// - when merging two releases, all linked file sets will be moved to the target release.
    ///
    pub async fn import(
        &self,
        system_id: i64,
        source_path: PathBuf,
        dat_file_path: Option<PathBuf>,
        file_type: FileType,
        item_type: Option<ItemType>,
    ) -> Result<(), Error> {
        tracing::info!(
            system_id = system_id,
            source_path = ?source_path,
            dat_file_path = ?dat_file_path,
            file_type = ?file_type,
            "Starting mass import process...");

        let mut context = MassImportContext::new(
            source_path,
            dat_file_path,
            file_type,
            item_type,
            system_id,
            self.repository_manager.clone(),
            self.settings.clone(),
        );
        let pipeline = Pipeline::<MassImportContext>::new();
        tracing::info!("Mass import process completed.");
        let res = pipeline.execute(&mut context).await;

        Ok(())
    }
}
