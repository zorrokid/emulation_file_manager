use std::{collections::HashMap, path::PathBuf};

use dat_file_parser::{DatFile, DatFileParserOps};
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
    pub path: PathBuf,
    pub release_name: String,
    pub software_title_name: String,
    // This can be passed directly to create_file_set in file_import service to proceed with
    // actual creation of file sets
    pub file_set: Option<FileSetImportModel>,
    pub status: ImportItemStatus,
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
    pub failed_files: Vec<PathBuf>,
}

impl MassImportContext {
    pub fn new(source_path: PathBuf, dat_file_path: Option<PathBuf>) -> Self {
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
        }
    }

    pub fn with_fs_ops(
        source_path: PathBuf,
        fs_ops: Box<dyn FileSystemOps>,
        dat_file_path: Option<PathBuf>,
        dat_file_parser_ops: Box<dyn DatFileParserOps>,
        file_import_ops: Box<dyn FileImportOps>,
        reader_factory_fn: Box<SendReaderFactoryFn>,
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
}
