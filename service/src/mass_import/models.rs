use std::{collections::HashMap, path::PathBuf};

use core_types::{FileType, ReadFile, item_type::ItemType};

use crate::{
    error::Error,
    file_import::model::FileSetImportModel,
    mass_import::{
        with_dat::context::{DatFileMassImportState, DatImportItem},
        with_files_only::context::FilesOnlyMassImportState,
    },
};
use domain::naming_conventions::no_intro::DatFile;

#[derive(Debug, Clone)]
pub struct MassImportInput {
    pub source_path: PathBuf,
    pub dat_file_path: Option<PathBuf>,
    pub file_type: FileType,
    pub item_type: Option<ItemType>,
    pub system_id: i64,
}

#[derive(Debug, Clone)]
pub struct MassImportSyncEvent {
    pub file_set_name: String,
    pub status: FileSetImportStatus,
}

#[derive(Debug, Clone)]
pub struct FileImportResult {
    pub read_ok_files: Vec<PathBuf>,
    pub read_failed_files: Vec<PathBuf>,
    pub dir_scan_errors: Vec<Error>,
    pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
    pub import_results: Vec<FileSetImportResult>,
}

#[derive(Debug, Clone)]
pub struct DatFileMassImportResult {
    pub dat_import_items: Vec<DatImportItem>,
    pub dat_file: Option<DatFile>,
    pub result: FileImportResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSetImportStatus {
    Success,
    SucessWithWarnings(Vec<String>), // Warning message
    Failed(String),                  // Error message
}

#[derive(Debug, Clone)]
pub struct FileSetImportResult {
    pub status: FileSetImportStatus,
    pub file_set_id: Option<i64>,
}

impl From<DatFileMassImportState> for DatFileMassImportResult {
    fn from(state: DatFileMassImportState) -> Self {
        DatFileMassImportResult {
            dat_import_items: state.import_items,
            dat_file: state.dat_file,
            result: FileImportResult {
                read_ok_files: state.read_ok_files,
                read_failed_files: state.read_failed_files,
                dir_scan_errors: state.dir_scan_errors,
                file_metadata: state.file_metadata,
                import_results: state.import_results,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilesOnlyMassImportResult {
    pub imported_file_sets: Vec<FileSetImportModel>,
    pub result: FileImportResult,
}

impl From<FilesOnlyMassImportState> for FilesOnlyMassImportResult {
    fn from(state: FilesOnlyMassImportState) -> Self {
        FilesOnlyMassImportResult {
            imported_file_sets: state.import_items,
            result: FileImportResult {
                read_ok_files: state.read_ok_files,
                read_failed_files: state.read_failed_files,
                dir_scan_errors: state.dir_scan_errors,
                file_metadata: state.file_metadata,
                import_results: state.import_results,
            },
        }
    }
}
