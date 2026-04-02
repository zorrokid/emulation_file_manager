use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    dat_game_status_service::DatGameFileSetStatus,
    file_set::FileSetServiceOps,
    mass_import::{
        common_steps::context::{CommonMassImportState, MassImportContextOps, MassImportDeps},
        models::{MassImportInput, MassImportSyncEvent},
    },
};
use core_types::Sha1Checksum;
use dat_file_parser::DatFileParserOps;
use domain::naming_conventions::no_intro::{DatFile, DatGame, DatHeader};
use file_metadata::SendReaderFactoryFn;
use flume::Sender;

use crate::{
    file_import::{
        file_import_service_ops::FileImportServiceOps,
        model::FileSetImportModel,
    },
    file_system_ops::FileSystemOps,
};

#[derive(Debug)]
pub struct DatFileMassImportContext {
    pub deps: MassImportDeps,
    pub input: MassImportInput,
    pub state: DatFileMassImportState,
    pub ops: DatFileMassImportOps,
    pub progress_tx: Option<Sender<MassImportSyncEvent>>,
}

pub struct DatFileMassImportOps {
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub dat_file_parser_ops: Arc<dyn DatFileParserOps>,
    pub file_import_service_ops: Arc<dyn FileImportServiceOps>,
    pub reader_factory_fn: Arc<SendReaderFactoryFn>,
    pub file_set_service_ops: Arc<dyn FileSetServiceOps>,
}

impl std::fmt::Debug for DatFileMassImportOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MassImportOps").finish()
    }
}

#[derive(Default, Debug)]
pub struct DatFileMassImportState {
    pub common_state: CommonMassImportState,
    pub dat_file: Option<DatFile>,
    pub dat_file_id: Option<i64>,
    pub dat_game_statuses: Vec<DatGameFileSetStatus>,
}

impl MassImportContextOps for DatFileMassImportContext {
    fn common_state(&self) -> &CommonMassImportState {
        &self.state.common_state
    }
    fn common_state_mut(&mut self) -> &mut CommonMassImportState {
        &mut self.state.common_state
    }
    fn reader_factory_fn(&self) -> Arc<SendReaderFactoryFn> {
        self.ops.reader_factory_fn.clone()
    }

    fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
        self.ops.fs_ops.clone()
    }

    fn source_path(&self) -> &Path {
        &self.input.source_path
    }

    fn can_import_file_sets(&self) -> bool {
        // File set statuses has to be determined and the parsed dat file has to be available.
        !self.state.dat_game_statuses.is_empty() && self.state.dat_file.is_some()
    }

    fn get_import_file_sets(&self) -> Vec<FileSetImportModel> {
        let Some(dat_file) = &self.state.dat_file else {
            return Vec::new();
        };
        let sha1_to_file_map = self.scanned_files_by_sha1();
        self.state
            .dat_game_statuses
            .iter()
            .filter_map(|status| match status {
                DatGameFileSetStatus::NonExisting(game) => {
                    Some(super::build_file_set_import_model(game, &dat_file.header, &sha1_to_file_map, self))
                }
                _ => None,
            })
            .collect()
    }

    fn import_service_ops(&self) -> Arc<dyn FileImportServiceOps> {
        self.ops.file_import_service_ops.clone()
    }

    fn progress_tx(&self) -> &Option<Sender<MassImportSyncEvent>> {
        &self.progress_tx
    }
}

impl DatFileMassImportContext {
    pub fn new(
        deps: MassImportDeps,
        input: MassImportInput,
        ops: DatFileMassImportOps,
        progress_tx: Option<Sender<MassImportSyncEvent>>,
    ) -> Self {
        Self {
            deps,
            input,
            state: DatFileMassImportState::default(),
            ops,
            progress_tx,
        }
    }

    /// Builds a map from SHA1 checksum to local file path using the scanned file metadata.
    /// Used to look up whether a DAT game's required ROM is locally available.
    pub fn scanned_files_by_sha1(&self) -> HashMap<Sha1Checksum, PathBuf> {
        self.state
            .common_state
            .file_metadata
            .iter()
            .flat_map(|(path, metadata_entries)| {
                metadata_entries
                    .iter()
                    .map(move |entry| (entry.sha1_checksum, path.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {

    use core_types::{FileType, item_type::ItemType};
    use dat_file_parser::MockDatParser;
    use database::repository_manager::RepositoryManager;
    use file_metadata::create_mock_factory_with_test_data;

    use crate::{
        file_import::file_import_service_ops::MockFileImportServiceOps,
        file_set::mock_file_set_service::MockFileSetService,
        file_system_ops::mock::MockFileSystemOps,
    };

    use super::*;

    async fn create_test_context(
        dat_file: Option<DatFile>,
        input: Option<MassImportInput>,
    ) -> DatFileMassImportContext {
        let dat_file = dat_file.unwrap_or_else(|| DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![],
        });
        let input = input.unwrap_or_else(|| MassImportInput {
            source_path: PathBuf::from("/test"),
            dat_file_path: None,
            file_type: FileType::Rom,
            item_type: Some(ItemType::Cartridge),
            system_id: 42,
        });

        let file_set_service_ops = Arc::new(MockFileSetService::new());
        let ops = DatFileMassImportOps {
            fs_ops: Arc::new(MockFileSystemOps::new()),
            dat_file_parser_ops: Arc::new(MockDatParser::new(Ok(dat_file.clone().into()))),
            file_import_service_ops: Arc::new(MockFileImportServiceOps::new()),
            reader_factory_fn: create_mock_factory_with_test_data(vec![/* test data */]),
            file_set_service_ops,
        };
        let pool = Arc::new(database::setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let deps = MassImportDeps { repository_manager };
        let mut context = DatFileMassImportContext::new(deps, input, ops, None);
        context.state.dat_file = Some(dat_file);
        context
    }

    #[async_std::test]
    async fn test_get_non_failed_files() {
        let mut context = create_test_context(None, None).await;

        let common_state = CommonMassImportState {
            read_ok_files: vec![
                PathBuf::from("/test/file1.zip"),
                PathBuf::from("/test/file2.zip"),
                PathBuf::from("/test/file3.zip"),
            ],
            read_failed_files: vec![PathBuf::from("/test/file2.zip")],
            ..Default::default()
        };

        let state = DatFileMassImportState {
            common_state,
            ..Default::default()
        };
        context.state = state;
        let non_failed_files = context.get_non_failed_files();
        assert_eq!(non_failed_files.len(), 2);
        assert!(non_failed_files.contains(&PathBuf::from("/test/file1.zip")));
        assert!(non_failed_files.contains(&PathBuf::from("/test/file3.zip")));
    }
}
