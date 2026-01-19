use std::path::PathBuf;

use crate::file_system_ops::{FileSystemOps, StdFileSystemOps};

pub struct MassImportContext {
    pub source_path: PathBuf,
    pub fs_ops: Box<dyn FileSystemOps>,
}

impl MassImportContext {
    pub fn new(source_path: PathBuf) -> Self {
        let fs_ops: Box<dyn FileSystemOps> = Box::new(StdFileSystemOps);
        MassImportContext {
            source_path,
            fs_ops,
        }
    }

    pub fn with_fs_ops(source_path: PathBuf, fs_ops: Box<dyn FileSystemOps>) -> Self {
        MassImportContext {
            source_path,
            fs_ops,
        }
    }
}
