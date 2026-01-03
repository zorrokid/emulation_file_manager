use std::{collections::HashMap, sync::Arc};

use core_types::{ReadFile, Sha1Checksum};
use file_import::FileImportOps;

use crate::{
    error::Error,
    file_system_ops::FileSystemOps,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait CollectFileInfoContext {
    fn file_import_ops(&self) -> Arc<dyn FileImportOps>;
    fn set_file_info(&mut self, file_info: HashMap<Sha1Checksum, ReadFile>);
    fn file_path(&self) -> &std::path::PathBuf;
    fn fs_ops(&self) -> Arc<dyn FileSystemOps>;
}

pub struct CollectFileInfoStep<T: CollectFileInfoContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: CollectFileInfoContext> Default for CollectFileInfoStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: CollectFileInfoContext> CollectFileInfoStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: CollectFileInfoContext + Send + Sync> PipelineStep<T> for CollectFileInfoStep<T> {
    fn name(&self) -> &'static str {
        "collect_file_info"
    }

    fn should_execute(&self, context: &T) -> bool {
        let exists = context.fs_ops().exists(context.file_path());
        exists
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        let is_zip = match context.fs_ops().is_zip_archive(context.file_path()) {
            Ok(is_zip) => is_zip,
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_path = %context.file_path().display(),
                    "Failed to determine if file is a zip archive"
                );

                return StepAction::Abort(Error::IoError(format!(
                    "Failed to determine if file is a zip archive: {}",
                    err,
                )));
            }
        };

        let file_contents_res = if is_zip {
            context
                .file_import_ops()
                .read_zip_contents_with_checksums(context.file_path())
        } else {
            context
                .file_import_ops()
                .read_file_checksum(context.file_path())
        };

        match file_contents_res {
            Ok(file_contents) => {
                context.set_file_info(file_contents);

                StepAction::Continue
            }
            Err(err) => {
                tracing::error!(
                    error = %err,
                    file_path = %context.file_path().display(),
                    "Failed to read file contents and checksums"
                );

                StepAction::Abort(Error::IoError(format!(
                    "Failed to read file contents and checksums: {}",
                    err,
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::Path, sync::Arc};

    use core_types::{ReadFile, Sha1Checksum};
    use file_import::{FileImportOps, mock::MockFileImportOps};

    use crate::{
        file_import::common_steps::collect_file_info::{
            CollectFileInfoContext, CollectFileInfoStep,
        },
        file_system_ops::{FileSystemOps, mock::MockFileSystemOps},
        pipeline::pipeline_step::PipelineStep,
    };

    struct CollectFileInfoContextTestImpl {
        file_import_ops: Arc<dyn FileImportOps>,
        file_info: HashMap<Sha1Checksum, ReadFile>,
        file_path: std::path::PathBuf,
        fs_ops: Arc<dyn FileSystemOps>,
    }

    impl CollectFileInfoContext for CollectFileInfoContextTestImpl {
        fn file_import_ops(&self) -> Arc<dyn FileImportOps> {
            self.file_import_ops.clone()
        }

        fn set_file_info(&mut self, file_info: HashMap<Sha1Checksum, ReadFile>) {
            self.file_info = file_info;
        }

        fn file_path(&self) -> &std::path::PathBuf {
            &self.file_path
        }
        fn fs_ops(&self) -> Arc<dyn crate::file_system_ops::FileSystemOps> {
            self.fs_ops.clone()
        }
    }

    async fn initialize_context(
        file_path: &Path,
        file_import_ops: Arc<MockFileImportOps>,
        fs_ops: Arc<dyn FileSystemOps>,
    ) -> CollectFileInfoContextTestImpl {
        CollectFileInfoContextTestImpl {
            file_import_ops,
            file_info: HashMap::new(),
            file_path: file_path.to_path_buf(),
            fs_ops,
        }
    }

    #[async_std::test]
    async fn test_collect_file_info_step() {
        let path_str = "/test/roms/game.zip";
        let test_path = Path::new(path_str);
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
        let fs_ops = Arc::new(MockFileSystemOps::new());
        fs_ops.add_file(path_str);
        let mut context = initialize_context(test_path, file_import_ops, fs_ops).await;

        let step = CollectFileInfoStep::<CollectFileInfoContextTestImpl>::new();
        let action = step.execute(&mut context).await;

        assert!(matches!(action, super::StepAction::Continue));
        assert!(context.file_info.contains_key(&checksum));
        assert_eq!(
            context.file_info.get(&checksum).unwrap().file_name,
            "game.rom"
        );
    }
}
