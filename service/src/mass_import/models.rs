use std::{collections::HashMap, path::PathBuf};

use core_types::{FileType, ReadFile, item_type::ItemType};

use crate::{
    error::Error,
    mass_import::{
        common_steps::context::CommonMassImportState, with_dat::context::DatFileMassImportState,
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
    pub dat_file: Option<DatFile>,
    pub result: FileImportResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileSetImportStatus {
    Success,
    SuccessWithWarnings(Vec<String>),
    StillMissingFiles(Vec<String>), // Re-run attempted but no files became available
    Failed(String),
    AlreadyExists,
}

#[derive(Debug, Clone)]
pub struct FileSetImportResult {
    pub status: FileSetImportStatus,
    pub file_set_id: Option<i64>,
    pub file_set_name: String,
}

impl From<DatFileMassImportState> for DatFileMassImportResult {
    fn from(state: DatFileMassImportState) -> Self {
        DatFileMassImportResult {
            dat_file: state.dat_file,
            result: state.common_state.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilesOnlyMassImportResult {
    pub result: FileImportResult,
}

impl From<CommonMassImportState> for FileImportResult {
    fn from(state: CommonMassImportState) -> Self {
        FileImportResult {
            read_ok_files: state.read_ok_files,
            read_failed_files: state.read_failed_files,
            dir_scan_errors: state.dir_scan_errors,
            file_metadata: state.file_metadata,
            import_results: state.import_results,
        }
    }
}

impl From<FilesOnlyMassImportState> for FilesOnlyMassImportResult {
    fn from(state: FilesOnlyMassImportState) -> Self {
        FilesOnlyMassImportResult {
            result: state.common_state.into(),
        }
    }
}
