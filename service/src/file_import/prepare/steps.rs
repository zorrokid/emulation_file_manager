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
