use database::repository_manager::RepositoryManager;

pub struct FileSetDeletionService {
    repository_manager: RepositoryManager,
}

impl FileSetDeletionService {
    pub fn new(repository_manager: RepositoryManager) -> Self {
        Self { repository_manager }
    }

    //pub fn delete_file_set(&self, file_set_id: i64) -> Result<(), String> {
    // First fetch the file set with file infos from database

    // For each file info in file set, check if it is used in other file sets
    // If not, collect the file infos for deletion

    // Go through the file infos to delete
    // - check for file sync entries from db, if file is synced mark it for deletion
    // - check if file exists in local storage and delete it
    //   - if there's a failure in deletion, log it and continue
    //   - if the deletion was successful, remove the file info from db
    //   -- ensure that file_set_file_info link entry will be deleted also
    //   -- ensure that file_info_system link entry will be deleted also

    // When all file infos are processed, delete the file set from db
    // -- ensure that release_file_set link entry will be deleted also
    //}
}
