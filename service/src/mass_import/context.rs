use std::path::PathBuf;

use dat_file_parser::DatFileParserOps;

use crate::file_system_ops::{FileSystemOps, StdFileSystemOps};

pub struct MassImportContext {
    pub source_path: PathBuf,
    pub dat_file_path: Option<PathBuf>,
    pub fs_ops: Box<dyn FileSystemOps>,
    pub dat_file_parser_ops: Box<dyn DatFileParserOps>,
}

impl MassImportContext {
    pub fn new(source_path: PathBuf, dat_file_path: Option<PathBuf>) -> Self {
        let fs_ops: Box<dyn FileSystemOps> = Box::new(StdFileSystemOps);
        let dat_file_parser_ops: Box<dyn DatFileParserOps> =
            Box::new(dat_file_parser::DefaultDatParser);
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
            dat_file_parser_ops,
        }
    }

    pub fn with_fs_ops(
        source_path: PathBuf,
        fs_ops: Box<dyn FileSystemOps>,
        dat_file_path: Option<PathBuf>,
        dat_file_parser_ops: Box<dyn DatFileParserOps>,
    ) -> Self {
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
            dat_file_parser_ops,
        }
    }
}
