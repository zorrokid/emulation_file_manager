use std::{path::PathBuf, sync::Arc};

use core_types::FileSyncStatus;
use database::repository_manager::RepositoryManager;

use crate::{error::Error, view_models::Settings};

pub struct FileSetDeletionService {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
}

impl FileSetDeletionService {
    pub fn new(repository_manager: Arc<RepositoryManager>, settings: Arc<Settings>) -> Self {
        Self {
            repository_manager,
            settings,
        }
    }

    pub async fn delete_file_set(&self, file_set_id: i64) -> Result<(), Error> {
        // First check if file set is in use by any releases

        if self
            .repository_manager
            .get_file_set_repository()
            .is_in_use(file_set_id)
            .await?
        {
            return Err(Error::DbError(
                "File set is in use by one or more releases".to_string(),
            ));
        }

        // If not in use, then fetch the file set file infos from database

        let file_set_file_info = self
            .repository_manager
            .get_file_info_repository()
            .get_file_infos_by_file_set(file_set_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;

        // For each file in file set, check if it is used in other file sets
        // If not, collect the file for deletion

        let mut file_infos_for_deletion = vec![];

        let collection_root_dir = self.settings.collection_root_dir.clone();

        for file_info in file_set_file_info {
            let res = self
                .repository_manager
                .get_file_set_repository()
                .get_file_sets_by_file_info(file_info.id)
                .await?;
            if let [entry] = &res[..] {
                // exactly one entry
                if entry.id == file_set_id {
                    file_infos_for_deletion.push(file_info);
                }
            }
        }

        // Go through the file infos to delete
        for file_info in file_infos_for_deletion {
            // - check for file sync entries from db, if file is synced mark it for deletion
            let res = self
                .repository_manager
                .get_file_sync_log_repository()
                .get_logs_by_file_info(file_info.id)
                .await
                .map_err(|e| Error::DbError(e.to_string()))?;
            if let Some(entry) = res.last() {
                self.repository_manager
                    .get_file_sync_log_repository()
                    .add_log_entry(
                        file_info.id,
                        FileSyncStatus::DeletionPending,
                        "",
                        entry.cloud_key.as_str(),
                    )
                    .await
                    .map_err(|e| Error::DbError(e.to_string()))?;
            }

            // - check if file exists in local storage and delete it
            let file_path = PathBuf::from(&collection_root_dir)
                .join(file_info.file_type.dir_name())
                .join(file_info.archive_file_name);

            if file_path.exists() {
                if let Err(e) = std::fs::remove_file(&file_path) {
                    //   - if there's a failure in deletion, log it and continue
                    eprintln!(
                        "Failed to delete file: {:?}, error: {}. Continuing with next file.",
                        file_path, e
                    );
                } else {
                    //   - if the deletion was successful, remove the file info from db
                    //   TODO:
                    //   -- ensure that file_set_file_info link entry will be deleted also
                    //   -- ensure that file_info_system link entry will be deleted also
                    self.repository_manager
                        .get_file_info_repository()
                        .delete_file_info(file_info.id)
                        .await
                        .map_err(|e| Error::DbError(e.to_string()))?;
                }
            }
        }

        // unlink the file set from any releases

        // When all file infos are processed, delete the file set from db
        // TODO:
        // -- ensure that release_file_set link entry will be deleted also
        self.repository_manager
            .get_file_set_repository()
            .delete_file_set(file_set_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))?;
        Ok(())
    }
}
