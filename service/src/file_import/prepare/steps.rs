use core_types::Sha1Checksum;

use crate::{
    error::Error,
    file_import::{model::FileImportMetadata, prepare::context::PrepareFileImportContext},
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct CollectFileMetadataStep;

#[async_trait::async_trait]
impl PipelineStep<PrepareFileImportContext> for CollectFileMetadataStep {
    fn name(&self) -> &'static str {
        "collect_file_metadata"
    }

    async fn execute(&self, context: &mut PrepareFileImportContext) -> StepAction {
        let file_set_name = context
            .file_path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string());
        let file_set_file_name = context
            .file_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string());

        let zip_file_result = context.fs_ops.is_zip_archive(&context.file_path);

        match zip_file_result {
            Ok(is_zip_archive) => {
                if let (Some(file_set_name), Some(file_set_file_name)) =
                    (file_set_name, file_set_file_name)
                {
                    context.import_metadata = Some(FileImportMetadata {
                        file_set_name,
                        file_set_file_name,
                        is_zip_archive,
                    });
                    StepAction::Continue
                } else {
                    tracing::error!(
                        file_path = %context.file_path.display(),
                        "Failed to extract file set name or file name"
                    );
                    StepAction::Abort(Error::IoError(
                        "Failed to extract file set name or file name".into(),
                    ))
                }
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_path = %context.file_path.display(),
                    "Failed to check if file is zip archive"
                );

                StepAction::Abort(err)
            }
        }
    }
}

pub struct CollectFileContentStep;

#[async_trait::async_trait]
impl PipelineStep<PrepareFileImportContext> for CollectFileContentStep {
    fn name(&self) -> &'static str {
        "collect_file_metadata"
    }

    fn should_execute(&self, context: &PrepareFileImportContext) -> bool {
        context.import_metadata.is_some()
    }

    async fn execute(&self, context: &mut PrepareFileImportContext) -> StepAction {
        let is_zip = context.import_metadata.as_ref().unwrap().is_zip_archive;
        let file_contents_res = match is_zip {
            true => context
                .file_import_ops
                .read_zip_contents_with_checksums(&context.file_path),
            false => context
                .file_import_ops
                .read_file_checksum(&context.file_path),
        };

        match file_contents_res {
            Ok(file_contents) => {
                context.file_info = file_contents;
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_path = %context.file_path.display(),
                    "Failed to read file contents and checksums"
                );

                return StepAction::Abort(Error::IoError(format!(
                    "Failed to read file contents and checksums: {}",
                    err,
                )));
            }
        }

        StepAction::Continue
    }
}

pub struct ProcessFileContentStep;

#[async_trait::async_trait]
impl PipelineStep<PrepareFileImportContext> for ProcessFileContentStep {
    fn name(&self) -> &'static str {
        "process_file_content"
    }

    fn should_execute(&self, context: &PrepareFileImportContext) -> bool {
        !context.file_info.is_empty()
    }

    async fn execute(&self, context: &mut PrepareFileImportContext) -> StepAction {
        let file_checksums = context
            .file_info
            .keys()
            .cloned()
            .collect::<Vec<Sha1Checksum>>();

        let existing_files_res = context
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(file_checksums, context.file_type)
            .await;

        match existing_files_res {
            Ok(existing_files_file_info) => {
                tracing::info!(
                    existing_file_count = existing_files_file_info.len(),
                    "Fetched existing file info from repository"
                );
                context.existing_files = existing_files_file_info;

                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    "Failed to fetch existing file info from repository"
                );
                StepAction::Abort(Error::DbError("Failed to fetch existing file info".into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::Path, sync::Arc};

    use core_types::{FileType, ReadFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};
    use file_import::file_import_ops::mock::MockFileImportOps;

    use crate::{
        file_import::{model::FileImportMetadata, prepare::context::PrepareFileImportContext},
        file_system_ops::mock::MockFileSystemOps,
        pipeline::pipeline_step::PipelineStep,
    };

    #[async_std::test]
    async fn test_collect_file_metadata_step() {
        let test_path = Path::new("/test/roms/game.zip");
        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(test_path.to_string_lossy().to_string());
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let mut context = initialize_context(test_path, fs_ops, file_import_ops).await;

        let step = super::CollectFileMetadataStep;
        let action = step.execute(&mut context).await;

        assert!(matches!(action, super::StepAction::Continue));
        assert!(context.import_metadata.is_some());
        let metadata = context.import_metadata.unwrap();
        assert_eq!(metadata.file_set_name, "game");
        assert_eq!(metadata.file_set_file_name, "game.zip");
        assert!(metadata.is_zip_archive);
    }

    #[async_std::test]
    async fn test_collect_file_content_step() {
        let test_path = Path::new("/test/roms/game.zip");
        let fs_ops = Arc::new(MockFileSystemOps::new());
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let checksum: Sha1Checksum = [0u8; 20];
        file_import_ops.add_zip_file(
            checksum,
            ReadFile {
                file_name: "game.rom".into(),
                sha1_checksum: checksum,
                file_size: 1024,
            },
        );
        let mut context = initialize_context(test_path, fs_ops, file_import_ops).await;

        context.import_metadata = Some(FileImportMetadata {
            file_set_name: "game".into(),
            file_set_file_name: "game.zip".into(),
            is_zip_archive: true,
        });

        let step = super::CollectFileContentStep;
        let action = step.execute(&mut context).await;

        assert!(matches!(action, super::StepAction::Continue));
        assert!(context.file_info.contains_key(&checksum));
        assert_eq!(
            context.file_info.get(&checksum).unwrap().file_name,
            "game.rom"
        );
    }

    #[async_std::test]
    async fn test_process_file_content_step_file_not_in_db() {
        let test_path = Path::new("/test/roms/game.rom");
        let fs_ops = Arc::new(MockFileSystemOps::new());
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let checksum: Sha1Checksum = [0u8; 20];
        let mut context = initialize_context(test_path, fs_ops, file_import_ops).await;

        context.file_info.insert(
            checksum,
            ReadFile {
                file_name: "game.rom".into(),
                sha1_checksum: checksum,
                file_size: 2048,
            },
        );

        let step = super::ProcessFileContentStep;
        let action = step.execute(&mut context).await;

        assert!(matches!(action, super::StepAction::Continue));
        assert!(context.existing_files.is_empty());

        let imported_file_infos = context.get_imported_file_info();
        let import_content = imported_file_infos.content.get(&checksum).unwrap();
        assert!(import_content.existing_file_info_id.is_none());
        assert!(import_content.existing_archive_file_name.is_none());
    }

    #[async_std::test]
    async fn test_process_file_content_step_file_exists_in_db() {
        let test_path = Path::new("/test/roms/game.rom");
        let fs_ops = Arc::new(MockFileSystemOps::new());
        let file_import_ops = Arc::new(MockFileImportOps::new());
        let checksum: Sha1Checksum = [0u8; 20];
        let mut context = initialize_context(test_path, fs_ops, file_import_ops).await;
        let existing_file_archive_name = "some_cryptic_file_name";

        context
            .repository_manager
            .get_file_info_repository()
            .add_file_info(
                &checksum,
                2048,
                existing_file_archive_name,
                context.file_type,
            )
            .await
            .unwrap();

        context.file_info.insert(
            checksum,
            ReadFile {
                file_name: "game.rom".into(),
                sha1_checksum: checksum,
                file_size: 2048,
            },
        );

        let step = super::ProcessFileContentStep;
        let action = step.execute(&mut context).await;

        assert!(matches!(action, super::StepAction::Continue));
        assert!(context.existing_files.len() == 1);
        assert_eq!(context.existing_files[0].sha1_checksum, checksum);
        assert_eq!(context.existing_files[0].file_size, 2048);
        assert_eq!(
            context.existing_files[0].archive_file_name,
            existing_file_archive_name
        );

        let imported_file_infos = context.get_imported_file_info();

        let import_content = imported_file_infos.content.get(&checksum).unwrap();
        assert!(import_content.existing_file_info_id.is_some());
        let existing_file = import_content.existing_archive_file_name.as_ref().unwrap();
        assert_eq!(existing_file, existing_file_archive_name);
    }

    async fn initialize_context(
        path: &Path,
        fs_ops: Arc<MockFileSystemOps>,
        file_import_ops: Arc<MockFileImportOps>,
    ) -> PrepareFileImportContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));

        PrepareFileImportContext::new(
            repository_manager,
            path,
            FileType::Rom,
            fs_ops,
            file_import_ops,
        )
    }
}
