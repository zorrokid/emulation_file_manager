use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_std::channel::Sender;
use core_types::{FileType, ReadFile, item_type::ItemType};

use crate::{
    file_import::{file_import_service_ops::FileImportServiceOps, model::FileSetImportModel},
    file_system_ops::FileSystemOps,
    mass_import::{
        common_steps::context::{MassImportContextOps, MassImportDeps, SendReaderFactoryFn},
        models::{FileSetImportResult, MassImportSyncEvent},
        with_dat::context::ImportItemStatus,
    },
};

struct MassImportWithFilesOnlyOps {
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub file_import_service_ops: Arc<dyn FileImportServiceOps>,
    pub reader_factory_fn: Arc<SendReaderFactoryFn>,
}

#[derive(Debug, Clone)]
pub struct ImportItem {
    pub release_name: String,
    pub software_title_name: String,
    // This can be passed directly to create_file_set in file_import service to proceed with
    // actual creation of file sets.
    pub file_set: Option<FileSetImportModel>,
    pub status: ImportItemStatus,
}

struct MassImportWithFilesOnlyState {
    pub read_ok_files: Vec<std::path::PathBuf>,
    pub read_failed_files: Vec<std::path::PathBuf>,
    pub dir_scan_errors: Vec<crate::error::Error>,
    pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
    pub import_items: Vec<FileSetImportModel>,
    pub import_results: Vec<FileSetImportResult>,
}

pub struct MassImportWithFilesOnlyInput {
    pub source_path: PathBuf,
    pub file_type: FileType,
    pub item_type: Option<ItemType>,
    pub system_id: i64,
    pub source: String,
}

pub struct MassImportWithFilesOnlyContext {
    pub deps: MassImportDeps,
    pub input: MassImportWithFilesOnlyInput,
    pub state: MassImportWithFilesOnlyState,
    pub ops: MassImportWithFilesOnlyOps,
    pub progress_tx: Option<Sender<MassImportSyncEvent>>,
}

impl MassImportContextOps for MassImportWithFilesOnlyContext {
    fn reader_factory_fn(&self) -> Arc<SendReaderFactoryFn> {
        self.ops.reader_factory_fn.clone()
    }

    fn fs_ops(&self) -> Arc<dyn FileSystemOps> {
        self.ops.fs_ops.clone()
    }

    fn source_path(&self) -> &std::path::Path {
        &self.input.source_path
    }

    fn read_ok_files_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.state.read_ok_files
    }

    fn read_ok_files(&self) -> &Vec<PathBuf> {
        &self.state.read_ok_files
    }

    fn read_failed_files(&self) -> &Vec<PathBuf> {
        &self.state.read_failed_files
    }

    fn read_failed_files_mut(&mut self) -> &mut Vec<PathBuf> {
        &mut self.state.read_failed_files
    }

    fn dir_scan_errors(&mut self) -> &mut Vec<crate::error::Error> {
        &mut self.state.dir_scan_errors
    }

    fn file_metadata(&mut self) -> &mut HashMap<PathBuf, Vec<ReadFile>> {
        &mut self.state.file_metadata
    }

    fn get_import_file_sets(&self) -> Vec<FileSetImportModel> {
        self.state.import_items.clone()
    }

    fn import_service_ops(&self) -> Arc<dyn FileImportServiceOps> {
        self.ops.file_import_service_ops.clone()
    }

    fn import_results(&mut self) -> &mut Vec<FileSetImportResult> {
        // TODO
        unimplemented!()
    }

    fn progress_tx(&self) -> &Option<Sender<MassImportSyncEvent>> {
        &self.progress_tx
    }
}
