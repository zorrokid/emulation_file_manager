use core_types::{FileSetEqualitySpecs, FileSetFileEqualitySpecs};

use crate::{
    error::Error,
    mass_import::{
        common_steps::context::MassImportContextOps,
        with_files_only::context::FilesOnlyMassImportContext,
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct FilterExistingFileSetsStep;

/// Filter out files from file_metadata that already have file sets in the system so that they
/// won't be imported again.
#[async_trait::async_trait]
impl PipelineStep<FilesOnlyMassImportContext> for FilterExistingFileSetsStep {
    fn name(&self) -> &'static str {
        "filter_existing_file_sets"
    }

    fn should_execute(&self, context: &FilesOnlyMassImportContext) -> bool {
        !context.state.file_metadata.is_empty()
    }

    async fn execute(&self, context: &mut FilesOnlyMassImportContext) -> StepAction {
        let file_set_import_models = context.get_import_file_sets();
        let repository_manager = context.deps.repository_manager.clone();
        let file_type = context.input.file_type;
        for file_set_import_model in file_set_import_models {
            let mut file_set_file_info: Vec<FileSetFileEqualitySpecs> = Vec::new();

            let file_contents = file_set_import_model
                .import_files
                .iter()
                .flat_map(|import_file| import_file.content.values().clone())
                .collect::<Vec<_>>();

            for file in file_contents {
                file_set_file_info.push(FileSetFileEqualitySpecs {
                    file_name: file.file_name.clone(),
                    file_type,
                    sha1_checksum: file.sha1_checksum,
                });
            }

            let file_set_equality_specs = FileSetEqualitySpecs {
                file_set_name: file_set_import_model.file_set_name.clone(),
                file_set_file_name: file_set_import_model.file_set_file_name.clone(),
                file_type,
                source: context.input.source.clone(),
                file_set_file_info,
            };

            let existing_file_set_res = repository_manager
                .get_file_set_repository()
                .find_file_set(&file_set_equality_specs)
                .await;

            match existing_file_set_res {
                Ok(existing_file_set) => {
                    if existing_file_set.is_some() {
                        tracing::info!(
                            "File set '{}' already exists in the system, skipping import for file '{}'",
                            file_set_import_model.file_set_name,
                            file_set_import_model.import_files[0].path.display()
                        );
                        context
                            .state
                            .file_metadata
                            .remove(&file_set_import_model.import_files[0].path);
                    }
                }
                Err(e) => {
                    return StepAction::Abort(Error::DbError(format!(
                        "Error checking for existing file set: {}",
                        e
                    )));
                }
            }
        }

        StepAction::Continue
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, path::PathBuf, sync::Arc};

    use core_types::{FileType, ImportedFile, ReadFile, Sha1Checksum};
    use database::repository_manager::RepositoryManager;
    use file_metadata::create_mock_factory_with_test_data;

    use crate::{
        file_import::{
            file_import_service_ops::MockFileImportServiceOps, model::FileSetImportModel,
        },
        file_system_ops::mock::MockFileSystemOps,
        mass_import::{
            common_steps::context::MassImportDeps,
            with_files_only::context::{FilesOnlyMassImportInput, FilesOnlyMassImportOps},
        },
    };

    use super::*;

    #[async_std::test]
    async fn test_filter_existing_file_sets() {
        let repository_manager = database::setup_test_repository_manager().await;
        let system_id = add_system(repository_manager.clone()).await;
        let input: FilesOnlyMassImportInput = FilesOnlyMassImportInput {
            source_path: PathBuf::from("/test/path"),
            file_type: FileType::Rom,
            item_type: None,
            system_id,
            source: "test source".to_string(),
        };

        let deps = MassImportDeps { repository_manager };

        let ops = FilesOnlyMassImportOps {
            fs_ops: Arc::new(MockFileSystemOps::new()),
            file_import_service_ops: Arc::new(MockFileImportServiceOps::new()),
            reader_factory_fn: create_mock_factory_with_test_data(vec![]),
        };

        let mut context = FilesOnlyMassImportContext::new(deps, input, ops, None);

        // TODO: populate context with file metadata
        // and add some of the file sets to the database
        // assert that after executing the step, the file metadata for existing file sets is
        // removed from the context
        let file_1_sha1_checksum: Sha1Checksum = [0; 20];
        let file_1_path = PathBuf::from("/test/path/file.zip");

        let file_2_sha1_checksum: Sha1Checksum = [1; 20];
        let file_2_path = PathBuf::from("/test/path/file2.zip");

        let file_meta_data: HashMap<PathBuf, Vec<ReadFile>> = HashMap::from([
            (
                file_1_path.clone(),
                vec![ReadFile {
                    file_name: "file_1".to_string(),
                    file_size: 1024,
                    sha1_checksum: file_1_sha1_checksum,
                }],
            ),
            (
                file_2_path.clone(),
                vec![ReadFile {
                    file_name: "file_2".to_string(),
                    file_size: 2048,
                    sha1_checksum: file_2_sha1_checksum,
                }],
            ),
        ]);

        context.state.file_metadata = file_meta_data;

        insert_file_set_to_db(&context, &file_1_path, FileType::Rom).await;

        // Act

        let step = FilterExistingFileSetsStep;
        let action = step.execute(&mut context).await;

        // Assert
        assert!(matches!(action, StepAction::Continue));
        assert!(!context.state.file_metadata.contains_key(&file_1_path));
        assert!(context.state.file_metadata.contains_key(&file_2_path));

        // insert first file set to test database
        /*let import_files = context.get_import_file_sets();
        let file_1_file_set: &FileSetImportModel = import_files
            .iter()
            .find(|file_set| {
                file_set
                    .import_files
                    .iter()
                    .any(|import_file| import_file.path == file_1_path)
            })
            .unwrap();

        let file_1_files = file_1_file_set
            .import_files
            .iter()
            .flat_map(|import_file| import_file.content.values())
            .collect::<Vec<_>>();
        let file_set_repository = context.deps.repository_manager.get_file_set_repository();
        file_set_repository
            .add_file_set(
                &file_1_file_set.file_set_name,
                &file_1_file_set.file_set_file_name,
                &file_1_file_set.file_type,
                &context.input.source,
                &file_1_files
                    .iter()
                    .map(|file| ImportedFile {
                        original_file_name: file.file_name.clone(),
                        archive_file_name: "1234abcd".to_string(),
                        sha1_checksum: file.sha1_checksum,
                        file_size: 1024,
                    })
                    .collect::<Vec<_>>(),
                &[context.input.system_id],
            )
            .await
            .unwrap();*/
    }

    async fn add_system(repository_manager: Arc<RepositoryManager>) -> i64 {
        let system_repository = repository_manager.get_system_repository();
        system_repository.add_system("Test System").await.unwrap()
    }

    async fn insert_file_set_to_db(
        context: &FilesOnlyMassImportContext,
        file_path: &PathBuf,
        file_type: FileType,
    ) {
        let import_files = context.get_import_file_sets();
        let file_set: &FileSetImportModel = import_files
            .iter()
            .find(|file_set| {
                file_set
                    .import_files
                    .iter()
                    .any(|import_file| import_file.path == *file_path)
            })
            .unwrap();

        let files = file_set
            .import_files
            .iter()
            .flat_map(|import_file| import_file.content.values())
            .collect::<Vec<_>>();

        let file_set_repository = context.deps.repository_manager.get_file_set_repository();
        file_set_repository
            .add_file_set(
                &file_set.file_set_name,
                &file_set.file_set_file_name,
                &file_type,
                &context.input.source,
                &files
                    .iter()
                    .map(|file| ImportedFile {
                        original_file_name: file.file_name.clone(),
                        archive_file_name: "1234abcd".to_string(),
                        sha1_checksum: file.sha1_checksum,
                        file_size: file.file_size,
                    })
                    .collect::<Vec<_>>(),
                &[context.input.system_id],
            )
            .await
            .unwrap();
    }
}
