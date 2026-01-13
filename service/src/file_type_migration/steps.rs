use std::collections::{HashMap, HashSet};

use core_types::Sha1Checksum;

use crate::{
    error::Error,
    file_type_migration::{
        context::{FileTypeMigration, FileTypeMigrationContext},
        file_type_mapper::map_old_file_type_to_new,
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
                for file_set in file_sets.iter() {
                    let new_file_type = map_old_file_type_to_new(file_set.file_type);
                    if new_file_type != file_set.file_type {
                        tracing::info!(
                            id = file_set.id,
                            old_file_type = ?file_set.file_type,
                            new_file_type = ?new_file_type,
                            "FileSet mapping file type"
                        );
                        context.file_sets_to_migrate.insert(
                            file_set.id,
                            FileTypeMigration {
                                old_file_type: file_set.file_type,
                                new_file_type,
                            },
                        );
                    }
                }
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
        for (file_set_id, file_type_migration) in context.file_sets_to_migrate.iter() {
            tracing::info!(
                file_set_id = file_set_id,
                old_file_type = ?file_type_migration.old_file_type,
                new_file_type = ?file_type_migration.new_file_type,
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

                        let old_path = context.settings.get_file_path(
                            &file_type_migration.old_file_type,
                            &file.archive_file_name,
                        );

                        if !context.fs_ops.exists(&old_path) {
                            tracing::warn!(
                                file_id = file.id,
                                old_path = ?old_path,
                                "Old file path does not exist, skipping move"
                            );
                            context
                                .non_existing_local_file_sha1_checksums
                                .insert(file.sha1_checksum);
                            continue;
                        }

                        let new_path = context.settings.get_file_path(
                            &file_type_migration.new_file_type,
                            &file.archive_file_name,
                        );

                        if context.is_dry_run {
                            tracing::info!(
                                file_id = file.id,
                                "Dry run enabled - skipping move from {:?} to {:?}",
                                old_path,
                                new_path
                            );
                            context
                                .moved_local_file_sha1_checksums
                                .insert(file.sha1_checksum);

                            continue;
                        } else {
                            tracing::info!(
                                file_id = file.id,
                                "Moving file from {:?} to {:?}",
                                old_path,
                                new_path
                            );
                            let res = context.fs_ops.move_file(&old_path, &new_path);
                            match res {
                                Ok(_) => {
                                    tracing::info!(
                                        file_id = file.id,
                                        "Successfully moved file from {:?} to {:?}",
                                        old_path,
                                        new_path
                                    );
                                    context
                                        .moved_local_file_sha1_checksums
                                        .insert(file.sha1_checksum);
                                }
                                Err(err) => {
                                    tracing::error!(
                                        file_id = file.id,
                                        error = ?err,
                                        "Error moving file from {:?} to {:?}",
                                        old_path,
                                        new_path
                                    );
                                    return StepAction::Abort(Error::IoError(format!(
                                        "Error moving file: {}",
                                        err
                                    )));
                                }
                            }
                        }
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

        StepAction::Continue
    }
}

pub struct MoveCloudFilesStep;
// TODO: similar to MoveLocalFilesStep, implement moving files in cloud storage
// Check first from sync log if file has been synced to cloud storage already.
// Update sync log entry with new cloud key if moved successfully.

pub struct UpdateDatabaseStep;
// TODO: update database entries for file sets and file infos with new file types
