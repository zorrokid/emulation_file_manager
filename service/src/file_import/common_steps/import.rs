use std::{collections::HashMap, sync::Arc};

use core_types::{ImportedFile, Sha1Checksum};
use file_import::{FileImportModel, FileImportOps};

use crate::{
    error::Error,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait FileImportContextOps {
    fn set_imported_files(&mut self, imported_files: HashMap<Sha1Checksum, ImportedFile>);
    fn file_import_ops(&self) -> &Arc<dyn FileImportOps>;
    fn get_file_import_model(&self) -> FileImportModel;
    fn is_new_files_to_be_imported(&self) -> bool;
}

pub struct ImportFilesStep<T: FileImportContextOps> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: FileImportContextOps> ImportFilesStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: FileImportContextOps> Default for ImportFilesStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl<T: FileImportContextOps + Send + Sync> PipelineStep<T> for ImportFilesStep<T> {
    fn name(&self) -> &'static str {
        "import_files"
    }

    fn should_execute(&self, context: &T) -> bool {
        context.is_new_files_to_be_imported()
    }

    async fn execute(&self, context: &mut T) -> StepAction {
        let file_import_model = context.get_file_import_model();
        match context.file_import_ops().import(&file_import_model) {
            Ok(imported_files) => {
                context.set_imported_files(imported_files);
            }
            Err(err) => {
                tracing::error!("Error importing files: {}", err);
                return StepAction::Abort(Error::FileImportError(format!(
                    "Error importing files: {}",
                    err
                )));
            }
        }
        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::{FileType, ImportedFile, Sha1Checksum};
    use database::models::FileInfo;
    use file_import::mock::MockFileImportOps;

    use crate::{
        file_import::{
            common_steps::import::{FileImportContextOps, ImportFilesStep},
            model::{FileImportData, FileImportSource, ImportFileContent},
        },
        pipeline::pipeline_step::{PipelineStep, StepAction},
    };

    struct TestContext {
        file_import_data: FileImportData,
        file_import_ops: Arc<dyn file_import::FileImportOps>,
        imported_files: HashMap<Sha1Checksum, ImportedFile>,
        existing_files: Vec<FileInfo>,
    }

    impl FileImportContextOps for TestContext {
        fn set_imported_files(
            &mut self,
            imported_files: HashMap<Sha1Checksum, core_types::ImportedFile>,
        ) {
            self.imported_files = imported_files;
        }

        fn file_import_ops(&self) -> &Arc<dyn file_import::FileImportOps> {
            &self.file_import_ops
        }

        fn get_file_import_model(&self) -> file_import::FileImportModel {
            self.file_import_data
                .get_file_import_model(&self.existing_files)
        }
        fn is_new_files_to_be_imported(&self) -> bool {
            self.file_import_data
                .is_new_files_to_be_imported(&self.existing_files)
        }
    }

    fn create_file_import_data(
        selected_files: Vec<Sha1Checksum>,
        import_files: Vec<FileImportSource>,
    ) -> FileImportData {
        FileImportData {
            file_type: FileType::Rom,
            selected_files,
            output_dir: PathBuf::from("/imported/files"),
            import_files,
        }
    }

    fn create_test_context(
        file_import_ops: Arc<dyn file_import::FileImportOps>,
        file_import_data: FileImportData,
    ) -> TestContext {
        TestContext {
            file_import_data,
            file_import_ops,
            imported_files: HashMap::new(),
            existing_files: Vec::new(),
        }
    }

    #[async_std::test]
    async fn test_import_files_step_skipped() {
        let mock_ops = Arc::new(MockFileImportOps::new());
        let checksum: Sha1Checksum = [1u8; 20];
        let file_import_data = create_file_import_data(vec![checksum], vec![]);
        let context = create_test_context(mock_ops, file_import_data);
        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
            },
        );

        let step = ImportFilesStep::<TestContext>::new();
        assert!(!step.should_execute(&context));
    }

    #[async_std::test]
    async fn test_import_files_step_success() {
        let checksum: Sha1Checksum = [1u8; 20];

        // Setup mock before creating context
        let mock_ops = Arc::new(MockFileImportOps::new());
        mock_ops.add_imported_file(
            checksum,
            ImportedFile {
                original_file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
                archive_file_name: "archive123.zst".to_string(),
            },
        );

        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );
        let mut context = create_test_context(mock_ops, file_import_data);

        let step = ImportFilesStep::<TestContext>::new();
        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Continue));
        assert_eq!(context.imported_files.len(), 1);
        assert!(context.imported_files.contains_key(&checksum));
        let imported = context.imported_files.get(&checksum).unwrap();
        assert_eq!(imported.original_file_name, "game.rom");
        assert_eq!(imported.archive_file_name, "archive123.zst");
    }

    #[async_std::test]
    async fn test_import_files_step_failure() {
        let checksum: Sha1Checksum = [1u8; 20];

        // Setup mock to fail
        let mock_ops = Arc::new(MockFileImportOps::new());
        mock_ops.set_should_fail(true);

        let mut content = HashMap::new();
        content.insert(
            checksum,
            ImportFileContent {
                file_name: "game.rom".to_string(),
                sha1_checksum: checksum,
                file_size: 1024,
            },
        );

        let file_import_data = create_file_import_data(
            vec![checksum],
            vec![FileImportSource {
                path: PathBuf::from("/test/games.zip"),
                content,
            }],
        );
        let mut context = create_test_context(mock_ops, file_import_data);

        let step = ImportFilesStep::<TestContext>::new();
        let result = step.execute(&mut context).await;

        assert!(matches!(result, StepAction::Abort(_)));
        assert!(context.imported_files.is_empty());
    }
}
