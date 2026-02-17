use std::sync::Arc;

use database::repository_manager::RepositoryManager;

#[derive(Debug)]
pub enum SoftwareTitleServiceError {
    DatabaseError(String),
}

#[derive(Debug)]
pub struct SoftwareTitleService {
    repository_manager: Arc<RepositoryManager>,
}

impl SoftwareTitleService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    /// Merges multiple software titles into a single base software title. This involves:
    /// 1. For each software title to merge:
    ///    a. Retrieve all releases associated with the software title.
    ///    b. For each release, add the base software title to the release and remove the software title being merged.
    ///    c. Delete the software title being merged from the database
    ///    2. Commit the transaction to ensure all changes are applied atomically.
    ///
    ///    This method ensures that all releases previously associated with the merged software
    ///    titles are now associated with the base software title, and the merged software titles
    ///    are removed from the database.
    ///
    pub async fn merge(
        &self,
        base_software_title_id: i64,
        software_title_ids_to_merge: &[i64],
    ) -> Result<(), SoftwareTitleServiceError> {
        if software_title_ids_to_merge.is_empty() {
            return Ok(());
        }
        // Ensure the base software title is not included in the list of software titles to merge
        let software_title_ids_to_merge = software_title_ids_to_merge
            .iter()
            .filter(|id| **id != base_software_title_id)
            .cloned()
            .collect::<Vec<_>>();

        let mut transaction = self
            .repository_manager
            .begin_transaction()
            .await
            .map_err(|e| SoftwareTitleServiceError::DatabaseError(format!("{:?}", e)))?;

        let releases_for_base = self
            .repository_manager
            .get_release_repository()
            .get_releases_by_software_title_with_tx(base_software_title_id, &mut transaction)
            .await
            .map_err(|e| SoftwareTitleServiceError::DatabaseError(format!("{:?}", e)))?;

        let mut release_ids_for_base = releases_for_base.iter().map(|r| r.id).collect::<Vec<_>>();

        for id in software_title_ids_to_merge {
            let releases = self
                .repository_manager
                .get_release_repository()
                .get_releases_by_software_title_with_tx(id, &mut transaction)
                .await
                .map_err(|e| SoftwareTitleServiceError::DatabaseError(format!("{:?}", e)))?;

            let release_ids = releases.iter().map(|r| r.id).collect::<Vec<_>>();

            for release_id in &release_ids {
                self.repository_manager
                    .get_release_repository()
                    .remove_software_title_from_release_with_tx(*release_id, id, &mut transaction)
                    .await
                    .map_err(|e| SoftwareTitleServiceError::DatabaseError(format!("{:?}", e)))?;

                // Release may already be associated with the base software title
                // TODO: add test about this
                if !release_ids_for_base.contains(release_id) {
                    println!(
                        "Adding base software title {} to release {}",
                        base_software_title_id, release_id
                    );
                    self.repository_manager
                        .get_release_repository()
                        .add_software_title_to_release_with_tx(
                            *release_id,
                            base_software_title_id,
                            &mut transaction,
                        )
                        .await
                        .map_err(|e| {
                            SoftwareTitleServiceError::DatabaseError(format!("{:?}", e))
                        })?;
                    release_ids_for_base.push(*release_id);
                    println!(
                        "Base software title {} added to release {}",
                        base_software_title_id, release_id
                    );
                }
            }
            self.repository_manager
                .get_software_title_repository()
                .delete_software_title_with_tx(id, &mut transaction)
                .await
                .map_err(|e| SoftwareTitleServiceError::DatabaseError(format!("{:?}", e)))?;
        }

        transaction
            .commit()
            .await
            .map_err(|e| SoftwareTitleServiceError::DatabaseError(format!("{:?}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core_types::{FileType, ImportedFile, Sha1Checksum};

    use super::*;

    #[async_std::test]
    async fn test_merge_software_titles() {
        // Setup test database and repository manager
        let repository_manager = database::setup_test_repository_manager().await;
        let service = SoftwareTitleService::new(repository_manager.clone());

        // Create base software title and software titles to merge
        let base_software_title_id = repository_manager
            .get_software_title_repository()
            .add_software_title("Base Title", None)
            .await
            .unwrap();

        let software_title_id_to_merge1 = repository_manager
            .get_software_title_repository()
            .add_software_title("Title to Merge 1", None)
            .await
            .unwrap();

        let software_title_id_to_merge2 = repository_manager
            .get_software_title_repository()
            .add_software_title("Title to Merge 2", None)
            .await
            .unwrap();

        let system_id = repository_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let file_sha1: Sha1Checksum = [0; 20];
        let file = ImportedFile {
            original_file_name: "test.rom".to_string(),
            archive_file_name: "test.rom".to_string(),
            file_size: 1024,
            sha1_checksum: file_sha1,
        };

        let file_set_id = repository_manager
            .get_file_set_repository()
            .add_file_set(
                "Test File Set",
                "Test File Set",
                &FileType::Rom,
                "source",
                &[file],
                &[system_id],
            )
            .await
            .unwrap();

        // Create releases associated with the software titles to merge
        let release_id1 = repository_manager
            .get_release_repository()
            .add_release_full(
                "Release 1",
                &[software_title_id_to_merge1, software_title_id_to_merge2],
                &[file_set_id],
                &[system_id],
            )
            .await
            .unwrap();

        // In this case release includes the base software title and one of the titles to merge,
        // which should be handled correctly by the merge operation
        let release_id2 = repository_manager
            .get_release_repository()
            .add_release_full(
                "Release 2",
                &[base_software_title_id, software_title_id_to_merge1],
                &[file_set_id],
                &[system_id],
            )
            .await
            .unwrap();

        // Perform merge operation
        service
            .merge(
                base_software_title_id,
                &[software_title_id_to_merge1, software_title_id_to_merge2],
            )
            .await
            .unwrap();

        // Verify that the releases are now associated with the base software title and the merged titles are deleted
        let releases_for_base = repository_manager
            .get_release_repository()
            .get_releases_by_software_title(base_software_title_id)
            .await
            .unwrap();
        assert_eq!(releases_for_base.len(), 2);
        assert!(releases_for_base.iter().any(|r| r.id == release_id1));
        assert!(releases_for_base.iter().any(|r| r.id == release_id2));

        let merged_title1 = repository_manager
            .get_software_title_repository()
            .get_software_title(software_title_id_to_merge1)
            .await;
        assert!(merged_title1.is_err());

        let merged_title2 = repository_manager
            .get_software_title_repository()
            .get_software_title(software_title_id_to_merge2)
            .await;
        assert!(merged_title2.is_err());

        let base_title = repository_manager
            .get_software_title_repository()
            .get_software_title(base_software_title_id)
            .await
            .unwrap();
        assert_eq!(base_title.name, "Base Title");
    }
}
