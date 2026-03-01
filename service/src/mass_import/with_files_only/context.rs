use std::{collections::HashMap, path::PathBuf, sync::Arc};

use async_std::channel::Sender;
use core_types::{FileType, ReadFile, item_type::ItemType};
use domain::title_normalizer::file_name_to_canonical_software_title;
use file_metadata::SendReaderFactoryFn;

use crate::{
    file_import::{
        file_import_service_ops::FileImportServiceOps,
        model::{CreateReleaseParams, FileImportSource, FileSetImportModel, ImportFileContent},
    },
    file_system_ops::FileSystemOps,
    mass_import::{
        common_steps::context::{MassImportContextOps, MassImportDeps},
        models::{FileSetImportResult, MassImportSyncEvent},
        with_dat::context::ImportItemStatus,
    },
};

pub struct MassImportWithFilesOnlyOps {
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

#[derive(Default)]
pub struct MassImportWithFilesOnlyState {
    pub read_ok_files: Vec<std::path::PathBuf>,
    pub read_failed_files: Vec<std::path::PathBuf>,
    pub dir_scan_errors: Vec<crate::error::Error>,
    pub file_metadata: HashMap<PathBuf, Vec<ReadFile>>,
    pub import_items: Vec<FileSetImportModel>,
    pub import_results: Vec<FileSetImportResult>,
}

#[derive(Debug, Clone)]
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

impl MassImportWithFilesOnlyContext {
    pub fn new(
        deps: MassImportDeps,
        input: MassImportWithFilesOnlyInput,
        ops: MassImportWithFilesOnlyOps,
        progress_tx: Option<Sender<MassImportSyncEvent>>,
    ) -> Self {
        Self {
            deps,
            input,
            state: MassImportWithFilesOnlyState::default(),
            ops,
            progress_tx,
        }
    }
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
        let system_id = self.input.system_id;
        let file_type = self.input.file_type;
        let item_type = self.input.item_type;
        let source = self.input.source.clone();
        let mut file_import_sets: Vec<FileSetImportModel> = vec![];
        for (file_path, metadata) in self.state.file_metadata.iter() {
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
            let software_title = file_name_to_canonical_software_title(&file_name);

            let file_set_import_model = FileSetImportModel {
                file_set_name: file_path.file_stem().unwrap().to_string_lossy().to_string(),
                file_set_file_name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                import_files: vec![FileImportSource {
                    path: file_path.clone(),
                    content: metadata
                        .iter()
                        .map(|f| {
                            (
                                f.sha1_checksum,
                                ImportFileContent {
                                    file_name: f.file_name.clone(),
                                    file_size: f.file_size,
                                    sha1_checksum: f.sha1_checksum,
                                },
                            )
                        })
                        .collect(),
                }],
                selected_files: metadata.iter().map(|meta| meta.sha1_checksum).collect(),
                system_ids: vec![system_id],
                source: source.clone(),
                file_type,
                item_ids: vec![],
                item_types: item_type.into_iter().collect(),
                create_release: Some(CreateReleaseParams {
                    software_title_name: software_title,
                    release_name: "".to_string(), // TODO: improve later,
                }),
                dat_file_id: None,
            };
            file_import_sets.push(file_set_import_model);
        }
        file_import_sets
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

#[cfg(test)]
mod tests {

    use core_types::Sha1Checksum;
    use database::setup_test_repository_manager;
    use file_metadata::create_mock_factory_with_test_data;

    use crate::{
        file_import::file_import_service_ops::MockFileImportServiceOps,
        file_system_ops::mock::MockFileSystemOps,
    };

    use super::*;

    #[async_std::test]
    async fn test_get_import_file_sets() {
        // create context for test

        let deps = MassImportDeps {
            repository_manager: setup_test_repository_manager().await,
        };

        let ops = MassImportWithFilesOnlyOps {
            fs_ops: Arc::new(MockFileSystemOps::new()),
            file_import_service_ops: Arc::new(MockFileImportServiceOps::new()),
            reader_factory_fn: create_mock_factory_with_test_data(vec![]),
        };

        let file_name = "staff_of_karnath";
        let extension = "tap";
        let archive_extension = "zip";

        let archive_file_name = format!("{}.{}", file_name, archive_extension);
        let content_file_name = format!("{}.{}", file_name, extension);

        let file_path = PathBuf::from(format!("/test/path/{}", archive_file_name));

        let input = MassImportWithFilesOnlyInput {
            source_path: file_path.clone(),
            file_type: FileType::TapeImage,
            item_type: None,
            system_id: 1,
            source: "test source".to_string(),
        };

        let sha1_checksum: Sha1Checksum = [0u8; 20];

        let file_metadata = vec![ReadFile {
            file_name: content_file_name.clone(),
            file_size: 1024,
            sha1_checksum,
        }];

        let state = MassImportWithFilesOnlyState {
            read_ok_files: vec![],
            read_failed_files: vec![],
            dir_scan_errors: vec![],
            file_metadata: HashMap::from([(file_path.clone(), file_metadata)]),
            import_items: vec![],
            import_results: vec![],
        };

        let context = MassImportWithFilesOnlyContext {
            deps,
            ops,
            input,
            state,
            progress_tx: None,
        };

        let import_file_sets = context.get_import_file_sets();

        assert_eq!(import_file_sets.len(), 1);
        let file_set = &import_file_sets[0];
        assert_eq!(file_set.file_set_name, file_name);
        assert_eq!(file_set.file_set_file_name, archive_file_name);
        assert_eq!(file_set.import_files.len(), 1);

        let import_file = &file_set.import_files[0];
        assert_eq!(import_file.path, file_path);
        assert_eq!(import_file.content.len(), 1);

        let content = import_file.content.values().next().unwrap();
        assert_eq!(content.file_name, content_file_name);
        assert_eq!(content.file_size, 1024);
        assert_eq!(content.sha1_checksum, sha1_checksum);

        assert!(file_set.create_release.is_some());
        let create_release_params = file_set.create_release.as_ref().unwrap();
        assert_eq!(
            create_release_params.software_title_name,
            "Staff of Karnath"
        );
    }
}
