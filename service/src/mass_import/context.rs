use std::path::PathBuf;

use crate::file_system_ops::{FileSystemOps, StdFileSystemOps};

pub struct MassImportContext {
    pub source_path: PathBuf,
    pub dat_file_path: Option<PathBuf>,
    pub fs_ops: Box<dyn FileSystemOps>,
}

impl MassImportContext {
    pub fn new(source_path: PathBuf, dat_file_path: Option<PathBuf>) -> Self {
        let fs_ops: Box<dyn FileSystemOps> = Box::new(StdFileSystemOps);
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
        }
    }

    pub fn with_fs_ops(
        source_path: PathBuf,
        fs_ops: Box<dyn FileSystemOps>,
        dat_file_path: Option<PathBuf>,
    ) -> Self {
        MassImportContext {
            source_path,
            fs_ops,
            dat_file_path,
        }
    }
}
