use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::error::Error;

#[derive(Debug)]
pub struct ReleaseDeletionService {
    repository_manager: Arc<RepositoryManager>,
}

impl ReleaseDeletionService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn delete_release(&self, release_id: i64) -> Result<(), Error> {
        // 1. get file sets associated with the release

        let file_sets = self
            .repository_manager
            .get_file_set_repository()
            .get_file_sets_for_release(release_id)
            .await?;

        // 2. check which file sets are only associated with this release and can be deleted
        let mut file_sets_to_delete = Vec::new();
        let mut file_sets_to_keep = Vec::new();

        for file_set in file_sets {
            for file in &file_set.files {
                let associated_releases = self
                    .repository_manager
                    .get_release_repository()
                    .get_releases_for_file(file.id)
                    .await?;

                if associated_releases.len() == 1 && associated_releases[0].id == release_id {
                    file_sets_to_delete.push(file_set.clone());
                } else {
                    file_sets_to_keep.push(file_set.clone());
                }
            }
        }
    }
}
