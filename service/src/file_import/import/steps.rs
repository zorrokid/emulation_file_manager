use std::{collections::HashSet, path::PathBuf};

use core_types::ImportedFile;
use file_import::FileImportModel;

use crate::{
    error::Error,
    file_import::import::context::FileImportContext,
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct ImportFilesStep;
#[async_trait::async_trait]
impl PipelineStep<FileImportContext> for ImportFilesStep {
    fn name(&self) -> &'static str {
        "import_files"
    }
    async fn execute(&self, context: &mut FileImportContext) -> StepAction {
        let target_path = context.settings.get_file_type_path(&context.file_type);

        let new_files_file_name_filter = context
            .import_files
            .iter()
            .flat_map(|file| {
                file.content
                    .iter()
                    .filter_map(|(sha1_checksum, import_content)| {
                        if context.selected_files.contains(sha1_checksum)
                            && import_content.existing_file_info_id.is_none()
                        {
                            Some(import_content.file_name.clone())
                        } else {
                            None
                        }
                    })
            })
            .collect::<HashSet<String>>();

        let file_import_model = FileImportModel {
            file_path: context
                .import_files
                .iter()
                .map(|f| f.path.clone())
                .collect::<Vec<PathBuf>>(),
            output_dir: target_path.to_path_buf(),
            file_name: context.file_set_file_name.clone(),
            file_set_name: context.file_set_name.clone(),
            file_type: context.file_type,
            new_files_file_name_filter,
        };

        // TODO: mock file_import for testing
        let res = file_import::import(&file_import_model);
        match res {
            Ok(imported_files) => {
                context.imported_files = imported_files;
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

pub struct UpdateDatabaseStep;

#[async_trait::async_trait]
impl PipelineStep<FileImportContext> for UpdateDatabaseStep {
    fn name(&self) -> &'static str {
        "update_database"
    }
    async fn execute(&self, context: &mut FileImportContext) -> StepAction {
        let mut files_in_file_set: Vec<ImportedFile> =
            context.imported_files.values().cloned().collect();

        // add existing files that were selected
        context.import_files.iter().for_each(|file| {
            file.content
                .iter()
                .for_each(|(sha1_checksum, file_content)| {
                    if context.selected_files.contains(sha1_checksum)
                        && let Some(existing_achive_file_name) =
                            &file_content.existing_archive_file_name
                    {
                        files_in_file_set.push(ImportedFile {
                            original_file_name: file_content.file_name.clone(),
                            sha1_checksum: *sha1_checksum,
                            file_size: file_content.file_size,
                            archive_file_name: existing_achive_file_name.clone(),
                        });
                    }
                });
        });

        let result = context
            .repository_manager
            .get_file_set_repository()
            .add_file_set(
                &context.file_set_name,
                &context.file_set_file_name,
                &context.file_type,
                &context.source,
                &files_in_file_set,
                &context.system_ids,
            )
            .await;

        match result {
            Ok(id) => {
                tracing::info!(
                    "File set '{}' with id {} added to database",
                    context.file_set_name,
                    id
                );
                context.file_set_id = Some(id);
            }
            Err(err) => {
                tracing::error!(
                    "Error adding file set '{}' to database: {}",
                    context.file_set_name,
                    err
                );
                // TODO: if this fails, we should probably roll back the imported files because
                // they are now orphaned.
                return StepAction::Abort(Error::DbError(format!(
                    "Error adding file set to database: {}",
                    err
                )));
            }
        }

        StepAction::Continue
    }
}
