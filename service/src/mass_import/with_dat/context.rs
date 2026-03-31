use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    dat_game_status_service::DatGameFileSetStatus,
    file_import::model::CreateReleaseParams,
    file_set::FileSetServiceOps,
    mass_import::{
        common_steps::context::{CommonMassImportState, MassImportContextOps, MassImportDeps},
        models::{MassImportInput, MassImportSyncEvent},
    },
};
use core_types::{Sha1Checksum, sha1_from_hex_string};
use dat_file_parser::DatFileParserOps;
use domain::naming_conventions::no_intro::{DatFile, DatGame, DatHeader};
use file_metadata::SendReaderFactoryFn;
use flume::Sender;

use crate::{
    file_import::{
        file_import_service_ops::FileImportServiceOps,
        model::{DatImportExtras, FileImportSource, FileSetImportModel, ImportFileContent},
    },
    file_system_ops::FileSystemOps,
};

#[derive(Debug, Clone)]
pub struct DatImportItem {
    pub dat_game: DatGame,
    // This can be passed directly to create_file_set in file_import service to proceed with
    // actual creation of file sets.
    pub file_set: Option<FileSetImportModel>,
}

impl DatImportItem {
    pub fn new(dat_game: DatGame, file_set: Option<FileSetImportModel>) -> Self {
        DatImportItem { dat_game, file_set }
    }
}

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
    pub statuses: Vec<DatGameFileSetStatus>,
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
        !self.state.statuses.is_empty() && self.state.dat_file.is_some()
    }

    fn get_import_file_sets(&self) -> Vec<FileSetImportModel> {
        self.get_import_items()
            .into_iter()
            .filter_map(|item| item.file_set)
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

    pub fn build_sha1_to_file_map(&self) -> HashMap<Sha1Checksum, PathBuf> {
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

    fn get_import_item(
        &self,
        game: &DatGame,
        header: &DatHeader,
        sha1_to_file_map: &HashMap<Sha1Checksum, PathBuf>,
    ) -> DatImportItem {
        tracing::info!(game = game.name.as_str(), "Processing DAT game");

        let mut import_files: HashMap<PathBuf, Vec<ImportFileContent>> = HashMap::new();
        let mut selected_files: Vec<Sha1Checksum> = vec![];
        let mut missing_files: Vec<ImportFileContent> = vec![];

        for rom in &game.roms {
            let sha1_bytes_res: Sha1Checksum =
                sha1_from_hex_string(&rom.sha1).expect("Invalid SHA1 in DAT");

            if let Some(source_file) = sha1_to_file_map.get(&sha1_bytes_res) {
                tracing::info!(
                    rom_sha1 = rom.sha1.as_str(),
                    source_file = %source_file.display(),
                    "Matched ROM to source file"
                );
                selected_files.push(sha1_bytes_res);
                import_files
                    .entry(source_file.clone())
                    .or_default()
                    .push(ImportFileContent {
                        file_name: rom.name.clone(),
                        sha1_checksum: sha1_bytes_res,
                        file_size: rom.size,
                    });
            } else {
                tracing::warn!(
                    rom_sha1 = rom.sha1.as_str(),
                    "No matching source file found for ROM"
                );
                missing_files.push(ImportFileContent {
                    file_name: rom.name.clone(),
                    sha1_checksum: sha1_bytes_res,
                    file_size: rom.size,
                });
            }
        }

        let create_release_params = CreateReleaseParams {
            release_name: game.get_release_name(),
            software_title_name: game.get_software_title_name(),
        };

        let file_set_import_model = Some(FileSetImportModel {
            import_files: import_files
                .into_iter()
                .map(|(path, contents)| FileImportSource {
                    path,
                    content: contents.into_iter().map(|c| (c.sha1_checksum, c)).collect(),
                })
                .collect(),

            selected_files,

            system_ids: vec![self.input.system_id],
            file_type: self.input.file_type,

            source: header.get_source(),
            file_set_name: game.name.clone(),
            file_set_file_name: game.name.clone(),

            item_ids: vec![],
            item_types: self
                .input
                .item_type
                .map_or_else(Vec::new, |item_type| vec![item_type]),
            create_release: Some(create_release_params),
            dat_extras: Some(DatImportExtras {
                missing_files,
                dat_file_id: self.state.dat_file_id,
            }),
        });
        DatImportItem::new(game.clone(), file_set_import_model)
    }

    /// Builds the list of import items from the DAT file for those file sets
    /// that are marked as non-existing in the system. This is used to prepare
    /// the data for the actual import step.
    pub fn get_import_items(&self) -> Vec<DatImportItem> {
        let Some(dat_file) = &self.state.dat_file else {
            tracing::error!("Attempted to get import items but DAT file is not loaded");
            return Vec::new();
        };

        tracing::info!("Mapping DAT entries to import items...");
        let sha1_to_file_map = self.build_sha1_to_file_map();

        self.state
            .statuses
            .iter()
            .filter_map(|status| match status {
                DatGameFileSetStatus::NonExisting(dat_game) => {
                    Some(self.get_import_item(dat_game, &dat_file.header, &sha1_to_file_map))
                }
                DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
                    file_set_id,
                    game,
                    ..
                } => {
                    tracing::info!(
                        game = game.name.as_str(),
                        file_set_id = *file_set_id,
                        "Game has existing file set"
                    );
                    Some(self.get_import_item(game, &dat_file.header, &sha1_to_file_map))
                }
                _ => None,
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
