use database::models::FileInfo;

use crate::{
    error::Error,
    file_type_migration::{
        context::{FileTypeMigration, FileTypeMigrationContext},
        file_type_mapper::{map_old_file_type_to_item_type, map_old_file_type_to_new},
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
                    let item_type = map_old_file_type_to_item_type(file_set.file_type);
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
                                item_type,
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

pub struct CollectCloudFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for CollectCloudFileSetsStep {
    fn name(&self) -> &'static str {
        "collect_cloud_file_sets_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        // NOTE: at this point there are not so many file sets, so loading all into memory is
        let files = context
            .repository_manager
            .get_file_sync_log_repository()
            .get_all_synced_file_set_ids()
            .await;
        match files {
            Ok(file_set_ids) => {
                context.file_ids_synced_to_cloud = file_set_ids;
            }
            Err(err) => {
                tracing::error!(
                    error = ?err,
                    "Error fetching synced file set ids");
                return StepAction::Abort(Error::DbError(format!(
                    "Error fetching synced file set ids: {}",
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
                        if context.moved_local_file_ids.contains(&file.id) {
                            continue;
                        }

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
                            context.non_existing_local_file_ids.insert(file.id);
                            continue;
                        }

                        let new_path = context.settings.get_file_path(
                            &file_type_migration.new_file_type,
                            &file.archive_file_name,
                        );

                        if context.is_dry_run {
                            tracing::info!(
                                old_path = ?old_path,
                                new_path = ?new_path,
                                file_id = file.id,
                                "Dry run enabled - skipping move from {:?} to {:?}",
                                old_path,
                                new_path
                            );
                            context.moved_local_file_ids.insert(file.id);

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
                                    context.moved_local_file_ids.insert(file.id);
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

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for MoveCloudFilesStep {
    fn name(&self) -> &'static str {
        "move_cloud_files_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
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
                        if context.moved_cloud_file_ids.contains(&file.id) {
                            tracing::info!(file_id = file.id, "File already moved, skipping move");
                            continue;
                        }

                        if !context.file_ids_synced_to_cloud.contains(&file.id) {
                            tracing::info!(
                                file_id = file.id,
                                "File not synced to cloud, skipping move"
                            );
                            continue;
                        }

                        tracing::info!(
                            file_id = file.id,
                            sha1_checksum = ?file.sha1_checksum,
                            "Moving cloud file to new location based on new file type"
                        );

                        let old_cloud_key = file.generate_cloud_key();

                        let new_file = FileInfo {
                            id: file.id,
                            sha1_checksum: file.sha1_checksum,
                            file_size: file.file_size,
                            archive_file_name: file.archive_file_name.clone(),
                            file_type: file_type_migration.new_file_type,
                        };

                        let new_cloud_key = new_file.generate_cloud_key();

                        if context.is_dry_run {
                            tracing::info!(
                                old_cloud_key = ?old_cloud_key,
                                new_cloud_key = ?new_cloud_key,
                                file_id = file.id,
                                "Dry run enabled - skipping cloud move for sha1_checksum {:?}",
                                file.sha1_checksum
                            );
                            context.moved_cloud_file_ids.insert(file.id);

                            continue;
                        } else {
                            tracing::info!(
                                file_id = file.id,
                                "Moving cloud file from {:?} to {:?}",
                                old_cloud_key,
                                new_cloud_key
                            );

                            let res = context
                                .cloud_storage_ops
                                .move_file(&old_cloud_key, &new_cloud_key)
                                .await;

                            match res {
                                Ok(_) => {
                                    tracing::info!(
                                        file_id = file.id,
                                        "Successfully moved cloud file from {:?} to {:?}",
                                        old_cloud_key,
                                        new_cloud_key
                                    );
                                    context.moved_cloud_file_ids.insert(file.id);
                                }
                                Err(err) => {
                                    tracing::error!(
                                        file_id = file.id,
                                        error = ?err,
                                        "Error moving cloud file from {:?} to {:?}",
                                        old_cloud_key,
                                        new_cloud_key
                                    );
                                    return StepAction::Abort(Error::IoError(format!(
                                        "Error moving cloud file: {}",
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

// TODO: update database entries for file sets and file infos with new file types
pub struct UpdateFileInfosStep;

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for UpdateFileInfosStep {
    fn name(&self) -> &'static str {
        "update_file_infos_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        for (file_set_id, file_type_migration) in context.file_sets_to_migrate.iter() {
            tracing::info!(
                file_set_id = file_set_id,
                old_file_type = ?file_type_migration.old_file_type,
                new_file_type = ?file_type_migration.new_file_type,
                "Updating FileInfo entries for FileSet"
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
                        if context.updated_file_info_ids.contains(&file.id) {
                            tracing::info!(
                                file_id = file.id,
                                "File already updated, skipping move"
                            );
                            continue;
                        }

                        tracing::info!(
                            file_id = file.id,
                            sha1_checksum = ?file.sha1_checksum,
                            "Updating FileInfo entry with new file type"
                        );

                        let result = context
                            .repository_manager
                            .get_file_info_repository()
                            .update_file_type(file.id, file_type_migration.new_file_type)
                            .await;

                        match result {
                            Ok(_) => {
                                tracing::info!(
                                    file_id = file.id,
                                    "Successfully updated FileInfo entry with new file type"
                                );
                                context.updated_file_info_ids.insert(file.id);
                            }
                            Err(err) => {
                                tracing::error!(
                                    file_id = file.id,
                                    error = ?err,
                                    "Error updating FileInfo entry with new file type"
                                );
                                return StepAction::Abort(Error::DbError(format!(
                                    "Error updating FileInfo: {}",
                                    err
                                )));
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

pub struct UpdateFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for UpdateFileSetsStep {
    fn name(&self) -> &'static str {
        "update_file_sets_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        for (file_set_id, file_type_migration) in context.file_sets_to_migrate.iter() {
            tracing::info!(
                file_set_id = file_set_id,
                old_file_type = ?file_type_migration.old_file_type,
                new_file_type = ?file_type_migration.new_file_type,
                "Updating FileSet entry with new file type"
            );

            let result = context
                .repository_manager
                .get_file_set_repository()
                .update_file_type(file_set_id, &file_type_migration.new_file_type)
                .await;

            match result {
                Ok(_) => {
                    tracing::info!(
                        file_set_id = file_set_id,
                        "Successfully updated FileSet entry with new file type"
                    );
                    context.updated_file_set_ids.insert(*file_set_id);
                }
                Err(err) => {
                    tracing::error!(
                        file_set_id = file_set_id,
                        error = ?err,
                        "Error updating FileSet entry with new file type"
                    );
                    return StepAction::Abort(Error::DbError(format!(
                        "Error updating FileSet: {}",
                        err
                    )));
                }
            }
        }

        StepAction::Continue
    }
}

pub struct AddItemsToFileSetsStep;

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for AddItemsToFileSetsStep {
    fn name(&self) -> &'static str {
        "add_items_to_file_sets_step"
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        for (file_set_id, file_type_migration) in context.file_sets_to_migrate.iter() {
            tracing::info!(
                file_set_id = file_set_id,
                old_file_type = ?file_type_migration.old_file_type,
                new_file_type = ?file_type_migration.new_file_type,
                "Updating FileSet entry with new file type"
            );

            if let Some(item_type) = &file_type_migration.item_type {
                tracing::info!(
                    file_set_id = file_set_id,
                    item_type = ?item_type,
                    "Adding ItemType to FileSet"
                );
                let result = context
                    .repository_manager
                    .get_file_set_repository()
                    .add_item_type_to_file_set(file_set_id, item_type)
                    .await;
            } else {
                tracing::warn!(
                    file_set_id = file_set_id,
                    "No ItemType mapped for this FileType, skipping adding ItemType to FileSet"
                );
                continue;
            }
        }

        StepAction::Continue
    }
}
