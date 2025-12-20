use std::{collections::HashMap, sync::Arc};

use core_types::{ReadFile, Sha1Checksum};
use file_import::FileImportOps;

use crate::{
    error::Error,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait CollectFileInfoContext {
    fn is_zip_archive(&self) -> Option<bool>;
    fn file_import_ops(&self) -> Arc<dyn FileImportOps>;
    fn set_file_info(&mut self, file_info: HashMap<Sha1Checksum, ReadFile>);
    fn file_path(&self) -> &std::path::PathBuf;
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
        context.is_zip_archive().is_some()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        let file_contents_res = if context.is_zip_archive().unwrap() {
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
        pipeline::pipeline_step::PipelineStep,
    };

    struct CollectFileInfoContextTestImpl {
        is_zip: Option<bool>,
        file_import_ops: Arc<dyn FileImportOps>,
        file_info: HashMap<Sha1Checksum, ReadFile>,
        file_path: std::path::PathBuf,
    }

    impl CollectFileInfoContext for CollectFileInfoContextTestImpl {
        fn is_zip_archive(&self) -> Option<bool> {
            self.is_zip
        }

        fn file_import_ops(&self) -> Arc<dyn FileImportOps> {
            self.file_import_ops.clone()
        }

        fn set_file_info(&mut self, file_info: HashMap<Sha1Checksum, ReadFile>) {
            self.file_info = file_info;
        }

        fn file_path(&self) -> &std::path::PathBuf {
            &self.file_path
        }
    }

    async fn initialize_context(
        file_path: &Path,
        file_import_ops: Arc<MockFileImportOps>,
        is_zip: bool,
    ) -> CollectFileInfoContextTestImpl {
        CollectFileInfoContextTestImpl {
            is_zip: Some(is_zip),
            file_import_ops,
            file_info: HashMap::new(),
            file_path: file_path.to_path_buf(),
        }
    }

    #[async_std::test]
    async fn test_collect_file_info_step() {
        let test_path = Path::new("/test/roms/game.zip");
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
        let mut context = initialize_context(test_path, file_import_ops, true).await;

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
