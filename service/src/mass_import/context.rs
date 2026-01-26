use std::{collections::HashMap, path::PathBuf};

use core_types::{FileType, ReadFile, Sha1Checksum, item_type::ItemType, sha1_from_hex_string};
use dat_file_parser::{DatFile, DatFileParserOps, DatGame, DatRom};
use file_import::FileImportOps;
use file_metadata::reader_factory::create_metadata_reader;

use crate::{
    file_import::model::{FileImportSource, FileSetImportModel, ImportFileContent},
    file_system_ops::{FileSystemOps, StdFileSystemOps},
};

/// Type alias for a Send-able metadata reader factory function
type SendReaderFactoryFn = dyn Fn(
        &std::path::Path,
    ) -> Result<Box<dyn file_metadata::FileMetadataReader>, file_metadata::FileMetadataError>
    + Send;

pub enum ImportItemStatus {
    Pending,
    Success,
    Failed(String), // Error message
}

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
    pub fn new(dat_game: DatGame) -> Self {
        let software_title_name = dat_game.name.clone();
        let release_name = dat_game.description.clone();
        ImportItem {
            dat_game,
            dat_roms_available: Vec::new(),
            dat_roms_missing: Vec::new(),
            release_name,
            software_title_name,
            file_set: None,
            status: ImportItemStatus::Pending,
        }
    }
}
pub struct MassImportContext {
    pub source_path: PathBuf,
    pub dat_file_path: Option<PathBuf>,
    pub fs_ops: Box<dyn FileSystemOps>,
    pub dat_file_parser_ops: Box<dyn DatFileParserOps>,
    pub file_import_ops: Box<dyn FileImportOps>,
    pub dat_file: Option<DatFile>,
    pub reader_factory_fn: Box<SendReaderFactoryFn>,
    pub import_items: Vec<ImportItem>,
    pub files: Vec<PathBuf>,
    pub failed_files: Vec<PathBuf>,
    pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
    pub file_type: FileType,
    pub item_type: Option<ItemType>,
    pub system_id: i64,
}

impl MassImportContext {
    pub fn new(
        source_path: PathBuf,
        dat_file_path: Option<PathBuf>,
        file_type: FileType,
        item_type: Option<ItemType>,
        system_id: i64,
    ) -> Self {
        let fs_ops: Box<dyn FileSystemOps> = Box::new(StdFileSystemOps);
        let dat_file_parser_ops: Box<dyn DatFileParserOps> =
            Box::new(dat_file_parser::DefaultDatParser);
        let file_import_ops: Box<dyn FileImportOps> = Box::new(file_import::StdFileImportOps);
        let reader_factory_fn: Box<SendReaderFactoryFn> = Box::new(create_metadata_reader);
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
            dat_file_parser_ops,
            dat_file: None,
            file_import_ops,
            reader_factory_fn,
            import_items: Vec::new(),
            failed_files: Vec::new(),
            files: Vec::new(),
            file_metadata: HashMap::new(),
            file_type,
            item_type,
            system_id,
        }
    }

    pub fn with_fs_ops(
        source_path: PathBuf,
        fs_ops: Box<dyn FileSystemOps>,
        dat_file_path: Option<PathBuf>,
        dat_file_parser_ops: Box<dyn DatFileParserOps>,
        file_import_ops: Box<dyn FileImportOps>,
        reader_factory_fn: Box<SendReaderFactoryFn>,
        file_type: FileType,
        item_type: Option<ItemType>,
        system_id: i64,
    ) -> Self {
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
            dat_file_parser_ops,
            dat_file: None,
            file_import_ops,
            reader_factory_fn,
            import_items: Vec::new(),
            failed_files: Vec::new(),
            files: Vec::new(),
            file_metadata: HashMap::new(),
            file_type,
            item_type,
            system_id,
        }
    }

    pub fn get_sha1_checksum_to_game_name_map(&self) -> HashMap<String, String> {
        let map: HashMap<String, String> = self
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

    pub fn get_non_failed_files(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|file| !self.failed_files.contains(file))
            .cloned()
            .collect()
    }

    pub fn build_sha1_to_file_map(&self) -> HashMap<Sha1Checksum, PathBuf> {
        self.file_metadata
            .iter()
            .flat_map(|(path, metadata_entries)| {
                metadata_entries
                    .iter()
                    .map(move |entry| (entry.sha1_checksum, path.clone()))
            })
            .collect()
    }

    pub fn get_import_items(&self) -> Vec<ImportItem> {
        if let Some(dat_file) = &self.dat_file {
            let mut import_items: Vec<ImportItem> = Vec::new();
            tracing::info!("Mapping DAT entries to import items...");

            let sha1_to_file_map = self.build_sha1_to_file_map();

            for game in &dat_file.games {
                tracing::info!(game = game.name.as_str(), "Processing DAT game");

                let mut import_item = ImportItem::new(game.clone());
                let mut import_files: HashMap<PathBuf, Vec<ImportFileContent>> = HashMap::new();
                for rom in &game.roms {
                    let sha1_bytes_res: Sha1Checksum =
                        sha1_from_hex_string(&rom.sha1).expect("Invalid SHA1 in DAT");

                    if let Some(source_file) = sha1_to_file_map.get(&sha1_bytes_res) {
                        tracing::info!(
                            rom_sha1 = rom.sha1.as_str(),
                            source_file = source_file.display().to_string().as_str(),
                            "Matched ROM to source file"
                        );
                        import_item.dat_roms_available.push(rom.clone());
                        import_files.entry(source_file.clone()).or_default().push(
                            ImportFileContent {
                                file_name: rom.name.clone(),
                                sha1_checksum: sha1_bytes_res,
                                file_size: rom.size,
                            },
                        );
                    } else {
                        tracing::warn!(
                            rom_sha1 = rom.sha1.as_str(),
                            "No matching source file found for ROM"
                        );
                        import_item.dat_roms_missing.push(rom.clone());
                    }
                }

                let item_types = self
                    .item_type
                    .map_or_else(Vec::new, |item_type| vec![item_type]);

                let import_files: Vec<FileImportSource> = import_files
                    .into_iter()
                    .map(|(path, contents)| FileImportSource {
                        path,
                        content: contents
                            .iter()
                            .map(|c| (c.sha1_checksum, c.clone()))
                            .collect(),
                    })
                    .collect();

                import_item.file_set = Some(FileSetImportModel {
                    import_files,
                    selected_files: vec![],

                    system_ids: vec![self.system_id],
                    file_type: self.file_type,

                    source: format!("{} {}", dat_file.header.name, dat_file.header.version),
                    file_set_name: game.name.clone(),
                    file_set_file_name: game.name.clone(),

                    item_ids: vec![],
                    item_types,
                });
                import_items.push(import_item);
            }
            import_items
        } else {
            tracing::warn!("No DAT file present in context; returning empty import items.");
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_import_items() {
        // Setup: Create a DAT file with two games
        let dat_file = DatFile {
            header: dat_file_parser::DatHeader {
                id: 1,
                name: "Test DAT".to_string(),
                description: "Test Description".to_string(),
                version: "1.0".to_string(),
                date: Some("2024-01-01".to_string()),
                author: "Test Author".to_string(),
                homepage: None,
                url: None,
                subset: None,
            },
            games: vec![
                DatGame {
                    name: "Game1".to_string(),
                    id: Some("1".to_string()),
                    cloneof: None,
                    cloneofid: None,
                    categories: vec![],
                    description: "First Game".to_string(),
                    roms: vec![DatRom {
                        name: "game1.rom".to_string(),
                        size: 1024,
                        crc: "12345678".to_string(),
                        md5: "d41d8cd98f00b204e9800998ecf8427e".to_string(),
                        sha1: "da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string(),
                        sha256: None,
                        status: None,
                        serial: None,
                        header: None,
                    }],
                    releases: vec![],
                },
                DatGame {
                    name: "Game2".to_string(),
                    id: Some("2".to_string()),
                    cloneof: None,
                    cloneofid: None,
                    categories: vec![],
                    description: "Second Game".to_string(),
                    roms: vec![
                        DatRom {
                            name: "game2a.rom".to_string(),
                            size: 2048,
                            crc: "87654321".to_string(),
                            md5: "098f6bcd4621d373cade4e832627b4f6".to_string(),
                            sha1: "2fd4e1c67a2d28fced849ee1bb76e7391b93eb12".to_string(),
                            sha256: None,
                            status: None,
                            serial: None,
                            header: None,
                        },
                        DatRom {
                            name: "game2b.rom".to_string(),
                            size: 512,
                            crc: "abcdef00".to_string(),
                            md5: "5d41402abc4b2a76b9719d911017c592".to_string(),
                            sha1: "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d".to_string(),
                            sha256: None,
                            status: None,
                            serial: None,
                            header: None,
                        },
                    ],
                    releases: vec![],
                },
            ],
        };

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
        let mut context = MassImportContext::new(
            PathBuf::from("/test"),
            None,
            FileType::Rom,
            Some(ItemType::Cartridge),
            42,
        );
        context.dat_file = Some(dat_file);
        context.file_metadata = file_metadata;

        // Execute: Get import items
        let import_items = context.get_import_items();

        // Verify: Should have 2 import items
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

        // Verify: Missing ROM
        assert_eq!(import_items[1].dat_roms_missing[0].name, "game2b.rom");
    }
}
