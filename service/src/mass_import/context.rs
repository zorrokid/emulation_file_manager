use std::{collections::HashMap, path::PathBuf};

use dat_file_parser::{DatFile, DatFileParserOps};
use file_import::FileImportOps;

use crate::file_system_ops::{FileSystemOps, StdFileSystemOps};

pub struct MassImportContext {
    pub source_path: PathBuf,
    pub dat_file_path: Option<PathBuf>,
    pub fs_ops: Box<dyn FileSystemOps>,
    pub dat_file_parser_ops: Box<dyn DatFileParserOps>,
    pub file_import_ops: Box<dyn FileImportOps>,
    pub dat_file: Option<DatFile>,
    pub files: Vec<PathBuf>,
    pub failed_files: Vec<(PathBuf, String)>,
}

impl MassImportContext {
    pub fn new(source_path: PathBuf, dat_file_path: Option<PathBuf>) -> Self {
        let fs_ops: Box<dyn FileSystemOps> = Box::new(StdFileSystemOps);
        let dat_file_parser_ops: Box<dyn DatFileParserOps> =
            Box::new(dat_file_parser::DefaultDatParser);
        let file_import_ops: Box<dyn FileImportOps> = Box::new(file_import::StdFileImportOps);
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
            dat_file_parser_ops,
            dat_file: None,
            files: Vec::new(),
            failed_files: Vec::new(),
            file_import_ops,
        }
    }

    pub fn with_fs_ops(
        source_path: PathBuf,
        fs_ops: Box<dyn FileSystemOps>,
        dat_file_path: Option<PathBuf>,
        dat_file_parser_ops: Box<dyn DatFileParserOps>,
        file_import_ops: Box<dyn FileImportOps>,
    ) -> Self {
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
            dat_file_parser_ops,
            dat_file: None,
            files: Vec::new(),
            failed_files: Vec::new(),
            file_import_ops,
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
