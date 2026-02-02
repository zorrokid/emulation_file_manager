use std::sync::Arc;

use async_std::channel::Sender;
use dat_file_parser::DatFileParserOps;
use database::repository_manager::RepositoryManager;
use file_metadata::reader_factory::create_metadata_reader;

use crate::{
    error::Error,
    file_import::{file_import_service_ops::FileImportServiceOps, service::FileImportService},
    file_system_ops::{FileSystemOps, StdFileSystemOps},
    mass_import::{
        context::{MassImportContext, MassImportOps, SendReaderFactoryFn},
        models::{MassImportInput, MassImportResult, MassImportSyncEvent},
    },
    pipeline::generic_pipeline::Pipeline,
    view_models::Settings,
};

pub struct MassImportService {
    fs_ops: Arc<dyn FileSystemOps>,
    dat_file_parser_ops: Arc<dyn DatFileParserOps>,
    file_import_service_ops: Arc<dyn FileImportServiceOps>,
    reader_factory_fn: Arc<SendReaderFactoryFn>,
}

impl std::fmt::Debug for MassImportService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MassImportService").finish()
    }
}

impl MassImportService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        let fs_ops: Arc<dyn FileSystemOps> = Arc::new(StdFileSystemOps);
        let dat_file_parser_ops: Arc<dyn DatFileParserOps> =
            Arc::new(dat_file_parser::DefaultDatParser);
        let reader_factory_fn: Arc<SendReaderFactoryFn> = Arc::new(create_metadata_reader);
        let file_import_service_ops: Arc<dyn FileImportServiceOps> = Arc::new(
            FileImportService::new(repository_manager.clone(), settings.clone()),
        );

        MassImportService::new_with_ops(
            fs_ops,
            dat_file_parser_ops,
            file_import_service_ops,
            reader_factory_fn,
        )
    }

    pub fn new_with_ops(
        fs_ops: Arc<dyn FileSystemOps>,
        dat_file_parser_ops: Arc<dyn DatFileParserOps>,
        file_import_service_ops: Arc<dyn FileImportServiceOps>,
        reader_factory_fn: Arc<SendReaderFactoryFn>,
    ) -> Self {
        MassImportService {
            fs_ops,
            dat_file_parser_ops,
            file_import_service_ops,
            reader_factory_fn,
        }
    }

    /// Starts the mass import process for the given system ID and source path.
    /// For each file or archive found in the source path, it will attempt to read metadata,
    /// match against the provided DAT file and import the files into the collection and
    /// database. It will create a file set for each file or archive successfully imported and a
    /// release with software title linked to the file sets.
    ///
    /// TODO: should we try to use existing software titles and releases if they already exist?
    ///
    /// For simplicity, let's start with creating new software titles and releases for each import.
    ///
    /// User can remove duplicated from UI. Theere will be also a functionality to merge software
    /// titles and releases in the future.
    /// - when merging two software titles, all linked releases will be moved to the target software title.
    /// - when merging two releases, all linked file sets will be moved to the target release.
    ///
    pub async fn import(
        &self,
        input: MassImportInput,
        progress_tx: Option<Sender<MassImportSyncEvent>>,
    ) -> Result<MassImportResult, Error> {
        tracing::info!(
            input = ?input,
            "Starting mass import process...");

        let ops = MassImportOps {
            fs_ops: self.fs_ops.clone(),
            dat_file_parser_ops: self.dat_file_parser_ops.clone(),
            file_import_service_ops: self.file_import_service_ops.clone(),
            reader_factory_fn: self.reader_factory_fn.clone(),
        };

        let mut context = MassImportContext::new(input, ops, progress_tx);
        let pipeline = Pipeline::<MassImportContext>::new();
        pipeline.execute(&mut context).await?;
        dbg!(&context.state);
        tracing::info!("Mass import process completed.");
        Ok(MassImportResult::from(context.state))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf};

    use super::*;
    use crate::{
        file_import::file_import_service_ops::{CreateMockState, MockFileImportServiceOps},
        file_system_ops::{SimpleDirEntry, mock::MockFileSystemOps},
        mass_import::{models::MassImportInput, test_utils::create_mock_reader_factory},
    };
    use async_std::channel;
    use core_types::{FileType, ReadFile, Sha1Checksum, sha1_bytes_to_hex_string};
    use dat_file_parser::{DatFile, DatGame, DatHeader, DatRom, MockDatParser};

    #[async_std::test]
    async fn test_mass_import_service_runs_pipeline_and_returns_result() {
        let mut fs_ops = MockFileSystemOps::new();
        fs_ops.add_entry(Ok(SimpleDirEntry {
            path: PathBuf::from("/mock/Test Game.zip"),
        }));

        let sha1_checksum: Sha1Checksum = [0xaa; 20];
        let sha1_checksum_string = sha1_bytes_to_hex_string(&sha1_checksum);

        let dat_game = DatGame {
            name: "Test Game".to_string(),
            description: "Test Game".to_string(),
            roms: vec![DatRom {
                name: "rom.bin".to_string(),
                sha1: sha1_checksum_string.clone(),
                size: 123,
                ..Default::default()
            }],
            ..Default::default()
        };
        let dat_file = DatFile {
            header: DatHeader {
                ..Default::default()
            },
            games: vec![dat_game],
        };

        let dat_file_parser_ops: Arc<dyn DatFileParserOps> =
            Arc::new(MockDatParser::new(Ok(dat_file)));
        let file_import_service_ops: Arc<dyn FileImportServiceOps> = Arc::new(
            MockFileImportServiceOps::with_create_mock(CreateMockState {
                file_set_id: 1,
                release_id: Some(1),
            }),
        );

        let mut metadata_by_path = HashMap::new();
        metadata_by_path.insert(
            PathBuf::from("/mock/Test Game.zip"),
            vec![ReadFile {
                file_name: "rom.bin".to_string(),
                sha1_checksum,
                file_size: 123,
            }],
        );
        let reader_factory_fn = Arc::new(create_mock_reader_factory(metadata_by_path, vec![]));

        let fs_ops = Arc::new(fs_ops);
        let service = MassImportService::new_with_ops(
            fs_ops,
            dat_file_parser_ops,
            file_import_service_ops,
            reader_factory_fn,
        );

        let input = MassImportInput {
            source_path: PathBuf::from("/mock"),
            dat_file_path: Some(PathBuf::from("/mock/datfile.dat")),
            file_type: FileType::Rom,
            item_type: None,
            system_id: 1,
        };

        // Optional progress channel (not asserted here, just exercised)
        let (tx, rx) = channel::unbounded();

        // Act
        let result = service.import(input, Some(tx)).await;

        // There should be one progress event for the one file set imported
        let event = rx.recv().await;

        assert!(
            event.is_ok(),
            "Should receive a progress event during import"
        );
        let event = event.unwrap();
        assert_eq!(
            event.file_set_name, "Test Game",
            "Progress event should have correct file set name"
        );

        // Assert
        assert!(
            result.is_ok(),
            "Mass import service should complete without error"
        );

        let import_result = result.unwrap();
        assert!(
            !import_result.import_results.is_empty(),
            "Import items should not be empty",
        );

        assert_eq!(
            import_result.import_results[0].status,
            crate::mass_import::models::FileSetImportStatus::Success,
            "First import result should be successful",
        );
    }
}
