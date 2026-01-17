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
pub struct CollectCloudFileSetsStep;
pub struct MoveLocalFilesStep;
pub struct MoveCloudFilesStep;
pub struct UpdateFileInfosStep;
pub struct UpdateFileSetsStep;
pub struct AddItemsToFileSetsStep;

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
                            item_type = ?item_type,
                            "FileSet mapping file type and item type"
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

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for MoveLocalFilesStep {
    fn name(&self) -> &'static str {
        "move_local_files_step"
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

#[async_trait::async_trait]
impl PipelineStep<FileTypeMigrationContext> for MoveCloudFilesStep {
    fn name(&self) -> &'static str {
        "move_cloud_files_step"
    }

    fn should_execute(&self, context: &FileTypeMigrationContext) -> bool {
        context.cloud_ops.is_some() && !context.file_ids_synced_to_cloud.is_empty()
    }

    async fn execute(&self, context: &mut FileTypeMigrationContext) -> StepAction {
        let cloud_ops = context.cloud_ops.as_ref().unwrap().clone();
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

                            let res = cloud_ops.move_file(&old_cloud_key, &new_cloud_key).await;

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

                        if context.is_dry_run {
                            tracing::info!(
                                file_id = file.id,
                                "Dry run enabled - skipping update of FileInfo entry with new file type"
                            );
                            context.updated_file_info_ids.insert(file.id);
                            continue;
                        } else {
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

            if context.is_dry_run {
                tracing::info!(
                    file_set_id = file_set_id,
                    "Dry run enabled - skipping update of FileSet entry with new file type"
                );
                context.updated_file_set_ids.insert(*file_set_id);
                continue;
            } else {
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
        }

        StepAction::Continue
    }
}

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
                if context.is_dry_run {
                    tracing::info!(
                        file_set_id = file_set_id,
                        "Dry run enabled - skipping adding ItemType to FileSet"
                    );
                    continue;
                } else {
                    tracing::info!(file_set_id = file_set_id, "Adding ItemType to FileSet");
                    let result = context
                        .repository_manager
                        .get_file_set_repository()
                        .add_item_type_to_file_set(file_set_id, item_type)
                        .await;
                    match result {
                        Ok(_) => {
                            tracing::info!(
                                file_set_id = file_set_id,
                                "Successfully added ItemType to FileSet"
                            );
                        }
                        Err(err) => {
                            tracing::error!(
                                file_set_id = file_set_id,
                                error = ?err,
                                "Error adding ItemType to FileSet"
                            );
                            return StepAction::Abort(Error::DbError(format!(
                                "Error adding ItemType to FileSet: {}",
                                err
                            )));
                        }
                    }
                }
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use cloud_storage::{CloudStorageOps, mock::MockCloudStorage};
    use core_types::{FileSyncStatus, FileType, ImportedFile, Sha1Checksum};
    use database::{repository_manager::RepositoryManager, setup_test_db};

    use crate::{
        file_system_ops::mock::MockFileSystemOps, settings_service::SettingsService,
        view_models::Settings,
    };

    use super::*;
    async fn setup_test_context(fs_ops: Option<MockFileSystemOps>) -> FileTypeMigrationContext {
        let pool = setup_test_db().await;
        let repository_manager = Arc::new(RepositoryManager::new(Arc::new(pool)));
        let settings = Arc::new(Settings {
            collection_root_dir: "/files".into(),
            ..Default::default()
        });
        let settings_service = Arc::new(SettingsService::new(repository_manager.clone()));

        let fs_ops = Arc::new(fs_ops.unwrap_or(MockFileSystemOps::new()));
        FileTypeMigrationContext::new(
            repository_manager,
            settings,
            settings_service,
            fs_ops,
            false,
        )
    }

    async fn insert_test_system(repository_manager: &RepositoryManager, name: &str) -> i64 {
        repository_manager
            .get_system_repository()
            .add_system(name)
            .await
            .unwrap()
    }

    async fn insert_test_file_set(
        repository_manager: &RepositoryManager,
        file_type: &FileType,
        file_sha1: Sha1Checksum,
        archive_file_name: String,
    ) -> i64 {
        let system_id = insert_test_system(repository_manager, "Test System").await;

        let imported_file = ImportedFile {
            sha1_checksum: file_sha1,
            file_size: 1234,
            archive_file_name,
            original_file_name: "original_test_file.rom".to_string(),
        };

        repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Test FileSet",
                "Test Description",
                file_type,
                "source",
                &[imported_file],
                &[system_id],
            )
            .await
            .unwrap()
    }

    #[async_std::test]
    async fn test_collect_file_sets_step_no_file_sets_to_migrate() {
        let mut context = setup_test_context(None).await;
        let step = CollectFileSetsStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert!(context.file_sets_to_migrate.is_empty());
    }

    #[async_std::test]
    async fn test_collect_file_sets_step_with_file_sets_to_migrate() {
        let mut context = setup_test_context(None).await;
        let repository_manager = context.repository_manager.clone();

        let file_1_checksum = Sha1Checksum::from([0; 20]);

        let file_set_id_1 = insert_test_file_set(
            &repository_manager,
            &FileType::ManualScan, // this will be migrated to FileType::Scan
            file_1_checksum,
            "123123.zst".to_string(),
        )
        .await;

        let file_2_checksum = Sha1Checksum::from([1; 20]);
        let _ = insert_test_file_set(
            &repository_manager,
            &FileType::Rom, // this will NOT be migrated
            file_2_checksum,
            "456456.zst".to_string(),
        )
        .await;

        let step = CollectFileSetsStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.file_sets_to_migrate.len(), 1);
        assert!(context.file_sets_to_migrate.contains_key(&file_set_id_1));
    }

    #[async_std::test]
    async fn test_collect_cloud_file_sets_step() {
        let mut context = setup_test_context(None).await;
        let repository_manager = context.repository_manager.clone();

        let file_info_id_1 = 1;
        let file_info_id_2 = 2;
        let _ = repository_manager
            .get_file_sync_log_repository()
            .add_log_entry(file_info_id_1, FileSyncStatus::UploadCompleted, "", "")
            .await
            .unwrap();
        let _ = repository_manager
            .get_file_sync_log_repository()
            .add_log_entry(file_info_id_2, FileSyncStatus::UploadPending, "", "")
            .await
            .unwrap();

        let step = CollectCloudFileSetsStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.file_ids_synced_to_cloud.len(), 1);
        assert!(context.file_ids_synced_to_cloud.contains(&file_info_id_1));
    }

    #[async_std::test]
    async fn test_move_local_files_step() {
        let fs_ops = MockFileSystemOps::new();
        let file_archive_name = "123123.zst".to_string();
        // only simulate that the other file exists in local file system
        fs_ops.add_file(format!("/files/manual_scan/{}", file_archive_name.clone()));
        let mut context = setup_test_context(Some(fs_ops)).await;
        let repository_manager = context.repository_manager.clone();
        let file_checksum = Sha1Checksum::from([0; 20]);

        // file in this file set will be moved
        let file_set_id = insert_test_file_set(
            &repository_manager,
            &FileType::ManualScan,
            file_checksum,
            file_archive_name.clone(),
        )
        .await;

        let file_info_id = repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap()[0]
            .id;

        let file_checksum_2 = Sha1Checksum::from([1; 20]);
        // file in this file set does not exist locally, so move will be skipped
        let file_set_id_2 = insert_test_file_set(
            &repository_manager,
            &FileType::Rom, // this will NOT be migrated
            file_checksum_2,
            "456456.zst".to_string(),
        )
        .await;

        context.file_sets_to_migrate.insert(
            file_set_id,
            FileTypeMigration {
                old_file_type: FileType::ManualScan,
                new_file_type: FileType::Scan,
                item_type: None,
            },
        );
        context.file_sets_to_migrate.insert(
            file_set_id_2,
            FileTypeMigration {
                old_file_type: FileType::Manual,
                new_file_type: FileType::Document,
                item_type: None,
            },
        );
        let step = MoveLocalFilesStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.moved_local_file_ids.len(), 1);
        assert!(context.moved_local_file_ids.contains(&file_info_id));
    }

    #[async_std::test]
    async fn test_move_cloud_files_step() {
        let mut context = setup_test_context(None).await;
        let repository_manager = context.repository_manager.clone();

        // create file set with file with file type to be migrated
        let archive_file_name = "123123.zst".to_string();
        let sha1_checksum = Sha1Checksum::from([0; 20]);
        let file_info = FileInfo {
            id: 1,
            sha1_checksum,
            file_size: 1234,
            archive_file_name: archive_file_name.clone(),
            file_type: FileType::ManualScan, // to be migrated to FileType::Scan
        };

        let file_set_id = insert_test_file_set(
            &repository_manager,
            &FileType::ManualScan,
            sha1_checksum,
            archive_file_name.clone(),
        )
        .await;

        // mark the file set for migration
        context.file_sets_to_migrate.insert(
            file_set_id,
            FileTypeMigration {
                old_file_type: FileType::ManualScan,
                new_file_type: FileType::Scan,
                item_type: None,
            },
        );

        // "upload" the file to mock cloud storage
        let file_path = context
            .settings
            .get_file_path(&FileType::ManualScan, &archive_file_name);
        let cloud_ops = Arc::new(MockCloudStorage::new());
        let cloud_key = file_info.generate_cloud_key();

        cloud_ops
            .upload_file(&file_path, &cloud_key, None)
            .await
            .unwrap();

        context.cloud_ops = Some(cloud_ops.clone());

        // mark the file as synced to cloud
        context.file_ids_synced_to_cloud.insert(file_info.id);

        let step = MoveCloudFilesStep;
        let action = step.execute(&mut context).await;
        assert!(matches!(action, StepAction::Continue));
        assert_eq!(context.moved_cloud_file_ids.len(), 1);
        assert!(context.moved_cloud_file_ids.contains(&file_info.id));

        let exists = cloud_ops.file_exists(&cloud_key).await.unwrap_or(false);
        assert!(!exists, "Old cloud key should not exist after move");

        let new_file = FileInfo {
            id: file_info.id,
            sha1_checksum: file_info.sha1_checksum,
            file_size: file_info.file_size,
            archive_file_name: file_info.archive_file_name.clone(),
            file_type: FileType::Scan,
        };

        let new_cloud_key = new_file.generate_cloud_key();
        let exists = cloud_ops.file_exists(&new_cloud_key).await.unwrap_or(false);
        assert!(exists, "New cloud key should exist after move");
    }
}
