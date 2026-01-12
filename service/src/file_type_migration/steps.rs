use std::collections::{HashMap, HashSet};

use core_types::{FileType, Sha1Checksum};

use crate::{
    error::Error,
    file_type_migration::{
        context::FileTypeMigrationContext, file_type_mapper::map_old_file_type_to_new,
    },
    pipeline::pipeline_step::{PipelineStep, StepAction},
};

pub struct CollectFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for CollectFileSetsStep {
    fn name(&self) -> &'static str {
        "collect_file_sets_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        // NOTE: at this point there are not so many file sets, so loading all into memory is
        // acceptable
        let file_sets = context
            .repository_manager
            .get_file_set_repository()
            .get_all_file_sets()
            .await;

        match file_sets {
            Ok(file_sets) => {
                let mut file_sets_to_migrate: HashMap<i64, FileType> = HashMap::new();

                for file_set in file_sets.iter() {
                    let new_file_type = map_old_file_type_to_new(file_set.file_type);
                    if new_file_type != file_set.file_type {
                        tracing::info!(
                            id = file_set.id,
                            old_file_type = ?file_set.file_type,
                            new_file_type = ?new_file_type,
                            "FileSet mapping file type"
                        );
                        file_sets_to_migrate.insert(file_set.id, new_file_type);
                    }
                }
                context.file_sets_to_migrate = file_sets_to_migrate;
            }
            Err(err) => {
                tracing::error!(
                    error = ?err,
                    "Error fetching file sets");
                return StepAction::Abort(Error::DbError(format!(
                    "Error fetching file sets: {}",
                    err
                )));
            }
        };

        StepAction::Continue
    }
}

pub struct MoveLocalFilesStep;

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for MoveLocalFilesStep {
    fn name(&self) -> &'static str {
        "collect_file_sets_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        let moved_file_sha1s: HashSet<Sha1Checksum> = HashSet::new();
        for (file_set_id, new_file_type) in context.file_sets_to_migrate.iter() {
            tracing::info!(
                file_set_id = file_set_id,
                new_file_type = ?new_file_type,
                "Moving local files for FileSet"
            );

            // Fetch files in the file set
            let files = context
                .repository_manager
                .get_file_info_repository()
                .get_file_infos_by_file_set(*file_set_id)
                .await;

            match files {
                Ok(files) => {
                    for file in files.iter() {
                        if moved_file_sha1s.contains(&file.sha1_checksum) {
                            continue;
                        }

                        // Here we would implement the logic to move the local file
                        // based on the new file type. This is a placeholder.
                        tracing::info!(
                            file_id = file.id,
                            sha1_checksum = ?file.sha1_checksum,
                            "Moving local file to new location based on new file type"
                        );

                        // Mark this file as moved
                        // moved_file_sha1s.insert(file_info.sha1_checksum.clone());
                    }
                }
                Err(err) => {
                    tracing::error!(
                        error = ?err,
                        "Error fetching files for FileSet id {}", file_set_id);
                    return StepAction::Abort(Error::DbError(format!(
                        "Error fetching files: {}",
                        err
                    )));
                }
            }
        }

        // Implementation for moving local files based on the new file types
        // This is a placeholder for the actual logic
        StepAction::Continue
    }
}

pub struct MoveCloudFilesStep;

pub struct UpdateDatabaseStep;
