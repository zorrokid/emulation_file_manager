use std::{collections::HashMap, path::PathBuf};

use core_types::{FileType, ReadFile, item_type::ItemType};
use dat_file_parser::DatFile;

use crate::{
    error::Error,
    mass_import::context::{FileSetImportResult, ImportItem, MassImportState},
};

#[derive(Debug, Clone)]
pub struct MassImportInput {
    pub source_path: PathBuf,
    pub dat_file_path: Option<PathBuf>,
    pub file_type: FileType,
    pub item_type: Option<ItemType>,
    pub system_id: i64,
}

#[derive(Debug, Clone)]
pub struct MassImportResult {
    pub import_items: Vec<ImportItem>,
    pub read_ok_files: Vec<PathBuf>,
    pub read_failed_files: Vec<PathBuf>,
    pub dir_scan_errors: Vec<Error>,
    pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
    pub dat_file: Option<DatFile>,
    pub import_results: Vec<FileSetImportResult>,
}

impl From<MassImportState> for MassImportResult {
    fn from(state: MassImportState) -> Self {
        MassImportResult {
            import_items: state.import_items,
            read_ok_files: state.read_ok_files,
            read_failed_files: state.read_failed_files,
            dir_scan_errors: state.dir_scan_errors,
            file_metadata: state.file_metadata,
            dat_file: state.dat_file,
            import_results: state.import_results,
        }
    }
}
