use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use core_types::{FileType, ReadFile, Sha1Checksum, item_type::ItemType};
use dat_file_parser::{DatFile, DatFileParserOps, DatGame, DatRom};
use file_import::FileImportOps;
use file_metadata::reader_factory::create_metadata_reader;

use crate::{
    file_import::model::FileSetImportModel,
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
}
