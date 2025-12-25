use std::sync::Arc;

use core_types::{FileType, Sha1Checksum};
use database::{models::FileInfo, repository_manager::RepositoryManager};

use crate::{
    error::Error,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub trait CheckExistingFilesContext {
    fn get_sha1_checksums(&self) -> Vec<Sha1Checksum>;
    fn file_type(&self) -> FileType;
    fn repository_manager(&self) -> Arc<RepositoryManager>;
    fn set_existing_files(&mut self, existing_files: Vec<FileInfo>);
}

pub struct CheckExistingFilesStep<T: CheckExistingFilesContext> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: CheckExistingFilesContext> Default for CheckExistingFilesStep<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: CheckExistingFilesContext> CheckExistingFilesStep<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: CheckExistingFilesContext + Send + Sync> PipelineStep<T> for CheckExistingFilesStep<T> {
    fn name(&self) -> &'static str {
        "check_existing_files"
    }

    fn should_execute(&self, context: &T) -> bool {
        !context.get_sha1_checksums().is_empty()
    }
    async fn execute(&self, context: &mut T) -> StepAction {
        let file_checksums = context.get_sha1_checksums();
        let existing_files_res = context
            .repository_manager()
            .get_file_info_repository()
            .get_file_infos_by_sha1_checksums(file_checksums, context.file_type())
            .await;

        match existing_files_res {
            Ok(existing_files_file_info) => {
                tracing::info!(
                    existing_file_count = existing_files_file_info.len(),
                    "Fetched existing file info from repository"
                );
                context.set_existing_files(existing_files_file_info);

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
    use std::{collections::HashMap, sync::Arc};

    use core_types::{FileType, ReadFile, Sha1Checksum};
    use database::{models::FileInfo, repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        file_import::common_steps::check_existing_files::{
            CheckExistingFilesContext, CheckExistingFilesStep,
        },
        pipeline::pipeline_step::PipelineStep,
    };

    struct TestContext {
        pub repository_manager: Arc<RepositoryManager>,
        pub file_type: FileType,
        pub existing_files: Vec<FileInfo>,
        pub file_info: HashMap<Sha1Checksum, ReadFile>,
    }

    impl CheckExistingFilesContext for TestContext {
        fn get_sha1_checksums(&self) -> Vec<Sha1Checksum> {
            self.file_info.keys().cloned().collect()
        }
        fn file_type(&self) -> FileType {
            self.file_type
        }
        fn repository_manager(&self) -> Arc<RepositoryManager> {
            self.repository_manager.clone()
        }
        fn set_existing_files(&mut self, existing_files: Vec<FileInfo>) {
            self.existing_files = existing_files;
        }
    }

    async fn initialize_context() -> TestContext {
        let pool = Arc::new(setup_test_db().await);
        let repository_manager = Arc::new(RepositoryManager::new(pool));

        TestContext {
            repository_manager,
            file_type: FileType::Rom,
            file_info: HashMap::new(),
            existing_files: Vec::new(),
        }
    }

    #[async_std::test]
    async fn test_check_existing_files_step_file_not_in_db() {
        let checksum: Sha1Checksum = [0u8; 20];
        let mut context = initialize_context().await;

        context.file_info.insert(
            checksum,
            ReadFile {
                file_name: "game.rom".into(),
                sha1_checksum: checksum,
                file_size: 2048,
            },
        );

        let step = CheckExistingFilesStep::<TestContext>::new();
        let action = step.execute(&mut context).await;

        assert!(matches!(action, super::StepAction::Continue));
        assert!(context.existing_files.is_empty());
    }

    #[async_std::test]
    async fn test_check_existing_files_step_file_exists_in_db() {
        let checksum: Sha1Checksum = [0u8; 20];
        let mut context = initialize_context().await;
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

        let step = CheckExistingFilesStep::<TestContext>::new();
        let action = step.execute(&mut context).await;

        assert!(matches!(action, super::StepAction::Continue));
        assert!(context.existing_files.len() == 1);
        assert_eq!(context.existing_files[0].sha1_checksum, checksum);
        assert_eq!(context.existing_files[0].file_size, 2048);
        assert_eq!(
            context.existing_files[0].archive_file_name,
            existing_file_archive_name
        );
    }
}
