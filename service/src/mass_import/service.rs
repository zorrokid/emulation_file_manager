use std::sync::Arc;

use async_std::channel::Sender;
use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    mass_import::{
        context::{MassImportContext, MassImportDependencies},
        models::{MassImportInput, MassImportResult, MassImportSyncEvent},
    },
    pipeline::generic_pipeline::Pipeline,
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
    /// match against the provided DAT file and import the files into the collection and
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
        input: MassImportInput,
        progress_tx: Option<Sender<MassImportSyncEvent>>,
    ) -> Result<MassImportResult, Error> {
        tracing::info!(
            input = ?input,
            "Starting mass import process...");

        let deps = MassImportDependencies {
            repository_manager: self.repository_manager.clone(),
            settings: self.settings.clone(),
        };

        let mut context = MassImportContext::new(input, deps, progress_tx);
        let pipeline = Pipeline::<MassImportContext>::new();
        pipeline.execute(&mut context).await?;
        tracing::info!("Mass import process completed.");
        Ok(MassImportResult::from(context.state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_mass_import_service() {}
}
