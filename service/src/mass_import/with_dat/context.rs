use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    dat_game_status_service::DatGameFileSetStatus,
    error::Error,
    file_import::model::CreateReleaseParams,
    file_set::FileSetServiceOps,
    mass_import::{
        common_steps::context::{MassImportContextOps, MassImportDeps, SendReaderFactoryFn},
        models::{FileSetImportResult, MassImportInput, MassImportSyncEvent},
    },
};
use async_std::channel::Sender;
use core_types::{ReadFile, Sha1Checksum, sha1_from_hex_string};
use dat_file_parser::DatFileParserOps;
use database::repository_manager::RepositoryManager;
use domain::naming_conventions::no_intro::{DatFile, DatGame, DatHeader, DatRom};

use crate::{
    file_import::{
        file_import_service_ops::FileImportServiceOps,
        model::{FileImportSource, FileSetImportModel, ImportFileContent},
    },
    file_system_ops::FileSystemOps,
    view_models::Settings,
};

#[derive(Debug, Clone)]
pub enum ImportItemStatus {
    Pending,
    Success,
    Failed(String), // Error message
}

#[derive(Debug, Clone)]
pub struct ImportItem {
    pub dat_game: DatGame,
    pub dat_roms_available: Vec<DatRom>,
    pub dat_roms_missing: Vec<DatRom>,
    pub release_name: String,
    pub software_title_name: String,
    // This can be passed directly to create_file_set in file_import service to proceed with
    // actual creation of file sets.
    pub file_set: Option<FileSetImportModel>,
    pub status: ImportItemStatus,
}

impl ImportItem {
    pub fn new(
        dat_game: DatGame,
        file_set: Option<FileSetImportModel>,
        dat_roms_available: Vec<DatRom>,
        dat_roms_missing: Vec<DatRom>,
    ) -> Self {
        let software_title_name = dat_game.name.clone();
        let release_name = dat_game.description.clone();
        ImportItem {
            dat_game,
            dat_roms_available,
            dat_roms_missing,
            release_name,
            software_title_name,
            file_set,
            status: ImportItemStatus::Pending,
        }
    }
}

#[derive(Debug)]
pub struct MassImportContext {
    pub deps: MassImportDeps,
    pub input: MassImportInput,
    pub state: MassImportState,
    pub ops: MassImportOps,
    pub progress_tx: Option<Sender<MassImportSyncEvent>>,
}

pub struct MassImportOps {
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub dat_file_parser_ops: Arc<dyn DatFileParserOps>,
    pub file_import_service_ops: Arc<dyn FileImportServiceOps>,
    pub reader_factory_fn: Arc<SendReaderFactoryFn>,
    pub file_set_service_ops: Arc<dyn FileSetServiceOps>,
}

impl std::fmt::Debug for MassImportOps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MassImportOps").finish()
    }
}

#[derive(Default, Debug)]
pub struct MassImportState {
    pub import_items: Vec<ImportItem>,
    pub read_ok_files: Vec<PathBuf>,
    pub read_failed_files: Vec<PathBuf>,
    pub dir_scan_errors: Vec<Error>,
    pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
    pub dat_file: Option<DatFile>,
    pub dat_file_id: Option<i64>,
    pub import_results: Vec<FileSetImportResult>,
    pub game_file_set_statuses: Vec<DatGameFileSetStatus>,
    pub statuses: Vec<DatGameFileSetStatus>,
}

pub struct MassImportDependencies {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

impl MassImportContextOps for MassImportContext {
    fn reader_factory_fn(&self) -> Arc<SendReaderFactoryFn> {
        self.ops.reader_factory_fn.clone()
    }

    fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
        self.ops.fs_ops.clone()
    }

    fn source_path(&self) -> &Path {
        &self.input.source_path
    }

    fn read_ok_files_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.state.read_ok_files
    }

    fn read_ok_files(&self) -> &Vec<PathBuf> {
        &self.state.read_ok_files
    }

    fn read_failed_files(&self) -> &Vec<PathBuf> {
        &self.state.read_failed_files
    }

    fn read_failed_files_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.state.read_failed_files
    }

    fn dir_scan_errors(&mut self) -> &mut Vec<Error> {
        &mut self.state.dir_scan_errors
    }

    fn file_metadata(&mut self) -> &mut HashMap<PathBuf, Vec<ReadFile>> {
        &mut self.state.file_metadata
    }

    fn get_import_file_sets(&self) -> Vec<FileSetImportModel> {
        self.get_import_items()
            .iter()
            .filter_map(|item| item.file_set.clone())
            .collect()
    }

    fn import_service_ops(&self) -> Arc<dyn FileImportServiceOps> {
        self.ops.file_import_service_ops.clone()
    }

    fn import_results(&mut self) -> &mut Vec<FileSetImportResult> {
        &mut self.state.import_results
    }

    fn progress_tx(&self) -> &Option<Sender<MassImportSyncEvent>> {
        &self.progress_tx
    }
}

impl MassImportContext {
    pub fn new(
        deps: MassImportDeps,
        input: MassImportInput,
        ops: MassImportOps,
        progress_tx: Option<Sender<MassImportSyncEvent>>,
    ) -> Self {
        Self {
            deps,
            input,
            state: MassImportState::default(),
            ops,
            progress_tx,
        }
    }

    pub fn get_sha1_checksum_to_game_name_map(&self) -> HashMap<String, String> {
        let map: HashMap<String, String> = self
            .state
            .dat_file
            .as_ref()
            .map(|dat_file| {
                dat_file
                    .games
                    .iter()
                    .flat_map(|game| {
                        game.roms
                            .iter()
                            .map(|rom| (rom.sha1.clone(), game.name.clone()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        map
    }

    pub fn build_sha1_to_file_map(&self) -> HashMap<Sha1Checksum, PathBuf> {
        self.state
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
    ) -> ImportItem {
        tracing::info!(game = game.name.as_str(), "Processing DAT game");

        let mut import_files: HashMap<PathBuf, Vec<ImportFileContent>> = HashMap::new();

        let mut available_roms: Vec<domain::naming_conventions::no_intro::DatRom> = vec![];
        let mut missing_roms: Vec<domain::naming_conventions::no_intro::DatRom> = vec![];
        for rom in &game.roms {
            let sha1_bytes_res: Sha1Checksum =
                sha1_from_hex_string(&rom.sha1).expect("Invalid SHA1 in DAT");

            if let Some(source_file) = sha1_to_file_map.get(&sha1_bytes_res) {
                tracing::info!(
                    rom_sha1 = rom.sha1.as_str(),
                    source_file = source_file.display().to_string().as_str(),
                    "Matched ROM to source file"
                );
                available_roms.push(rom.clone());
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
                missing_roms.push(rom.clone());
            }
        }

        let selected_files: Vec<Sha1Checksum> = available_roms
            .iter()
            .filter_map(|rom| sha1_from_hex_string(&rom.sha1).ok())
            .collect();

        let game: domain::naming_conventions::no_intro::DatGame = game.clone();

        let create_release_params = CreateReleaseParams {
            release_name: game.get_release_name(),
            software_title_name: game.get_software_title_name(),
        };

        let file_set = Some(FileSetImportModel {
            import_files: import_files
                .into_iter()
                .map(|(path, contents)| FileImportSource {
                    path,
                    content: contents
                        .iter()
                        .map(|c| (c.sha1_checksum, c.clone()))
                        .collect(),
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
            dat_file_id: self.state.dat_file_id,
        });
        ImportItem::new(game.clone(), file_set, available_roms, missing_roms)
    }

    pub fn get_import_items(&self) -> Vec<ImportItem> {
        let non_existing_games = self
            .state
            .statuses
            .iter()
            .filter(|status| matches!(status, DatGameFileSetStatus::NonExisting(_)))
            .map(|status| match status {
                DatGameFileSetStatus::NonExisting(dat_game) => dat_game.clone(),
                _ => unreachable!(),
            });

        self.state.dat_file.as_ref().map_or(Vec::new(), |dat_file| {
            let mut import_items: Vec<ImportItem> = Vec::new();
            tracing::info!("Mapping DAT entries to import items...");
            let sha1_to_file_map = self.build_sha1_to_file_map();
            non_existing_games.for_each(|game| {
                import_items.push(self.get_import_item(&game, &dat_file.header, &sha1_to_file_map));
            });
            import_items
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use core_types::{FileType, item_type::ItemType};
    use dat_file_parser::MockDatParser;
    use file_metadata::{FileMetadataError, FileMetadataReader, MockFileMetadataReader};

    use crate::{
        file_import::file_import_service_ops::MockFileImportServiceOps,
        file_set::mock_file_set_service::MockFileSetService,
        file_system_ops::mock::MockFileSystemOps,
    };

    use super::*;

    async fn create_test_context(
        dat_file: Option<DatFile>,
        input: Option<MassImportInput>,
    ) -> MassImportContext {
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
        // Mock factory always returns the same mock reader
        let mock_factory: Arc<SendReaderFactoryFn> = Arc::new(
            |_path: &Path| -> Result<Box<dyn FileMetadataReader>, FileMetadataError> {
                Ok(Box::new(MockFileMetadataReader {
                    metadata: vec![/* test data */],
                }))
            },
        );

        let file_set_service_ops = Arc::new(MockFileSetService::new());
        let ops = MassImportOps {
            fs_ops: Arc::new(MockFileSystemOps::new()),
            dat_file_parser_ops: Arc::new(MockDatParser::new(Ok(dat_file.clone().into()))),
            file_import_service_ops: Arc::new(MockFileImportServiceOps::new()),
            reader_factory_fn: mock_factory,
            file_set_service_ops,
        };
        let pool = Arc::new(database::setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let deps = MassImportDeps { repository_manager };
        let mut context = MassImportContext::new(deps, input, ops, None);
        context.state.dat_file = Some(dat_file);
        context
    }

    #[async_std::test]
    async fn test_get_import_items() {
        // Setup: Create a DAT file with two games
        let dat_game_1 = DatGame {
            name: "Game1".to_string(),
            id: Some("1".to_string()),
            description: "First Game".to_string(),
            roms: vec![DatRom {
                name: "game1.rom".to_string(),
                size: 1024,
                sha1: "da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let dat_game_2 = DatGame {
            name: "Game2".to_string(),
            id: Some("2".to_string()),
            description: "Second Game".to_string(),
            roms: vec![
                DatRom {
                    name: "game2a.rom".to_string(),
                    size: 2048,
                    sha1: "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12".to_string(),
                    ..Default::default()
                },
                DatRom {
                    name: "game2b.rom".to_string(),
                    size: 512,
                    sha1: "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let dat_game_3 = DatGame {
            name: "Game3".to_string(),
            id: Some("3".to_string()),
            roms: vec![DatRom {
                name: "game3.rom".to_string(),
                size: 4096,
                sha1: "e5e9fa1ba31ecd1ae84f75caaa474f3a663f05f4".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let dat_file = DatFile {
            header: DatHeader {
                name: "Test DAT".to_string(),
                version: "1.0".to_string(),
                ..Default::default()
            },
            games: vec![dat_game_1.clone(), dat_game_2.clone(), dat_game_3.clone()],
        };

        let statuses = vec![
            DatGameFileSetStatus::NonExisting(dat_game_1.clone()),
            DatGameFileSetStatus::NonExisting(dat_game_2.clone()),
            DatGameFileSetStatus::ExistingWithReleaseAndLinkedToDat {
                file_set_id: 1,
                game: dat_game_3.clone(),
            },
        ];

        // Setup: Create file metadata matching the first game and one ROM from the second
        let mut file_metadata = HashMap::new();
        let file1_path = PathBuf::from("/test/file1.zip");
        let file2_path = PathBuf::from("/test/file2.zip");

        file_metadata.insert(
            file1_path.clone(),
            vec![ReadFile {
                file_name: "game1.rom".to_string(),
                sha1_checksum: sha1_from_hex_string("da39a3ee5e6b4b0d3255bfef95601890afd80709")
                    .unwrap(),
                file_size: 1024,
            }],
        );

        file_metadata.insert(
            file2_path.clone(),
            vec![ReadFile {
                file_name: "game2a.rom".to_string(),
                sha1_checksum: sha1_from_hex_string("2fd4e1c67a2d28fced849ee1bb76e7391b93eb12")
                    .unwrap(),
                file_size: 2048,
            }],
        );

        // Create context with test data
        let input = MassImportInput {
            source_path: PathBuf::from("/test"),
            dat_file_path: None,
            file_type: FileType::Rom,
            item_type: Some(ItemType::Cartridge),
            system_id: 42,
        };
        let mut context = create_test_context(Some(dat_file.clone()), Some(input)).await;

        context.state.file_metadata = file_metadata;
        context.state.statuses = statuses;

        // Execute: Get import items
        let import_items = context.get_import_items();

        // Verify: Should have 2 import items, 3rd game is existing and should be filtered out
        assert_eq!(import_items.len(), 2);

        // Verify: First game should have all ROMs available
        assert_eq!(import_items[0].dat_game.name, "Game1");
        assert_eq!(import_items[0].dat_roms_available.len(), 1);
        assert_eq!(import_items[0].dat_roms_missing.len(), 0);
        assert_eq!(import_items[0].release_name, "First Game");
        assert_eq!(import_items[0].software_title_name, "Game1");
        assert!(import_items[0].file_set.is_some());

        let file_set_1 = import_items[0].file_set.as_ref().unwrap();
        assert_eq!(file_set_1.file_set_name, "Game1");
        assert_eq!(file_set_1.file_type, FileType::Rom);
        assert_eq!(file_set_1.system_ids, vec![42]);
        assert_eq!(file_set_1.import_files.len(), 1);
        assert_eq!(file_set_1.import_files[0].path, file1_path);
        assert_eq!(
            file_set_1.selected_files.len(),
            1,
            "selected_files should contain SHA1 of available ROM"
        );
        assert_eq!(
            file_set_1.selected_files[0],
            sha1_from_hex_string("da39a3ee5e6b4b0d3255bfef95601890afd80709").unwrap()
        );

        // Verify: Second game should have 1 ROM available and 1 missing
        assert_eq!(import_items[1].dat_game.name, "Game2");
        assert_eq!(import_items[1].dat_roms_available.len(), 1);
        assert_eq!(import_items[1].dat_roms_missing.len(), 1);
        assert_eq!(import_items[1].release_name, "Second Game");
        assert_eq!(import_items[1].software_title_name, "Game2");
        assert!(import_items[1].file_set.is_some());

        let file_set_2 = import_items[1].file_set.as_ref().unwrap();
        assert_eq!(file_set_2.file_set_name, "Game2");
        assert_eq!(file_set_2.import_files.len(), 1);
        assert_eq!(file_set_2.import_files[0].path, file2_path);
        assert_eq!(
            file_set_2.selected_files.len(),
            1,
            "selected_files should contain SHA1 of available ROM, not missing ones"
        );
        assert_eq!(
            file_set_2.selected_files[0],
            sha1_from_hex_string("2fd4e1c67a2d28fced849ee1bb76e7391b93eb12").unwrap()
        );

        // Verify: Missing ROM
        assert_eq!(import_items[1].dat_roms_missing[0].name, "game2b.rom");
    }

    #[async_std::test]
    async fn test_get_non_failed_files() {
        let mut context = create_test_context(None, None).await;

        let state = MassImportState {
            read_ok_files: vec![
                PathBuf::from("/test/file1.zip"),
                PathBuf::from("/test/file2.zip"),
                PathBuf::from("/test/file3.zip"),
            ],
            read_failed_files: vec![PathBuf::from("/test/file2.zip")],
            ..Default::default()
        };
        context.state = state;
        let non_failed_files = context.get_non_failed_files();
        assert_eq!(non_failed_files.len(), 2);
        assert!(non_failed_files.contains(&PathBuf::from("/test/file1.zip")));
        assert!(non_failed_files.contains(&PathBuf::from("/test/file3.zip")));
    }
}
