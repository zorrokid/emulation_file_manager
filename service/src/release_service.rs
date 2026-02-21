use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::error::Error;

#[derive(Debug)]
pub struct ReleaseService {
    repository_manager: Arc<RepositoryManager>,
}

impl ReleaseService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn add_release(
        &self,
        name: &str,
        software_title_ids: &[i64],
        file_set_ids: &[i64],
        system_ids: &[i64],
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_release_repository()
            .add_release_full(name, software_title_ids, file_set_ids, system_ids)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }

    pub async fn update_release(
        &self,
        id: i64,
        name: &str,
        software_title_ids: &[i64],
        file_set_ids: &[i64],
        system_ids: &[i64],
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_release_repository()
            .update_release_full(id, name, software_title_ids, file_set_ids, system_ids)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }

    pub async fn delete_release(&self, id: i64) -> Result<i64, Error> {
        self.repository_manager
            .get_release_repository()
            .delete_release(id)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use core_types::{FileType, ImportedFile, Sha1Checksum};

    use super::*;

    async fn add_system(repo_manager: &Arc<RepositoryManager>) -> i64 {
        repo_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap()
    }

    async fn add_software_title(repo_manager: &Arc<RepositoryManager>) -> i64 {
        repo_manager
            .get_software_title_repository()
            .add_software_title("Test Title", None)
            .await
            .unwrap()
    }

    async fn add_file_set(repo_manager: &Arc<RepositoryManager>, system_id: i64) -> i64 {
        let checksum: Sha1Checksum = [0u8; 20];
        let file = ImportedFile {
            original_file_name: "test.rom".to_string(),
            archive_file_name: "test.rom".to_string(),
            file_size: 512,
            sha1_checksum: checksum,
        };
        repo_manager
            .get_file_set_repository()
            .add_file_set("Test Set", "Test Set", &FileType::Rom, "src", &[file], &[system_id])
            .await
            .unwrap()
    }

    #[async_std::test]
    async fn add_release_returns_positive_id() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseService::new(repo_manager);
        let id = service.add_release("Game", &[], &[], &[]).await.unwrap();
        assert!(id > 0);
    }

    #[async_std::test]
    async fn add_release_links_all_associations() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseService::new(Arc::clone(&repo_manager));
        let system_id = add_system(&repo_manager).await;
        let title_id = add_software_title(&repo_manager).await;
        let file_set_id = add_file_set(&repo_manager, system_id).await;

        let id = service
            .add_release("Game", &[title_id], &[file_set_id], &[system_id])
            .await
            .unwrap();

        let systems = repo_manager
            .get_system_repository()
            .get_systems_by_release(id)
            .await
            .unwrap();
        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].id, system_id);

        let titles = repo_manager
            .get_software_title_repository()
            .get_software_titles_by_release(id)
            .await
            .unwrap();
        assert_eq!(titles.len(), 1);
        assert_eq!(titles[0].id, title_id);
    }

    #[async_std::test]
    async fn update_release_changes_name() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseService::new(Arc::clone(&repo_manager));
        let id = service.add_release("Old Name", &[], &[], &[]).await.unwrap();
        service
            .update_release(id, "New Name", &[], &[], &[])
            .await
            .unwrap();
        let release = repo_manager
            .get_release_repository()
            .get_release(id)
            .await
            .unwrap();
        assert_eq!(release.name, "New Name");
    }

    #[async_std::test]
    async fn update_release_replaces_system_associations() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseService::new(Arc::clone(&repo_manager));
        let system_a = add_system(&repo_manager).await;
        let system_b = repo_manager
            .get_system_repository()
            .add_system("System B")
            .await
            .unwrap();
        let id = service
            .add_release("Game", &[], &[], &[system_a])
            .await
            .unwrap();
        service
            .update_release(id, "Game", &[], &[], &[system_b])
            .await
            .unwrap();
        let systems = repo_manager
            .get_system_repository()
            .get_systems_by_release(id)
            .await
            .unwrap();
        assert_eq!(systems.len(), 1);
        assert_eq!(systems[0].id, system_b);
    }

    #[async_std::test]
    async fn delete_release_removes_record() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseService::new(Arc::clone(&repo_manager));
        let id = service.add_release("Game", &[], &[], &[]).await.unwrap();
        service.delete_release(id).await.unwrap();
        let result = repo_manager
            .get_release_repository()
            .get_release(id)
            .await;
        assert!(result.is_err());
    }
}
