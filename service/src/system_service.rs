use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::error::Error;

#[derive(Debug)]
pub struct SystemService {
    repository_manager: Arc<RepositoryManager>,
}

impl SystemService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn add_system(&self, name: &str) -> Result<i64, Error> {
        self.repository_manager
            .get_system_repository()
            .add_system(name)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }

    pub async fn update_system(&self, id: i64, name: &str) -> Result<i64, Error> {
        self.repository_manager
            .get_system_repository()
            .update_system(id, &name.to_string())
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }

    pub async fn delete_system(&self, id: i64) -> Result<(), Error> {
        self.repository_manager
            .get_system_repository()
            .delete_system(id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn add_system_returns_positive_id() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = SystemService::new(repo_manager);
        let id = service.add_system("Commodore 64").await.unwrap();
        assert!(id > 0);
    }

    #[async_std::test]
    async fn add_system_persists_name() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = SystemService::new(Arc::clone(&repo_manager));
        let id = service.add_system("Commodore 64").await.unwrap();
        let system = repo_manager
            .get_system_repository()
            .get_system(id)
            .await
            .unwrap();
        assert_eq!(system.name, "Commodore 64");
    }

    #[async_std::test]
    async fn update_system_changes_name() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = SystemService::new(Arc::clone(&repo_manager));
        let id = service.add_system("Commodore 64").await.unwrap();
        service.update_system(id, "Amiga 500").await.unwrap();
        let system = repo_manager
            .get_system_repository()
            .get_system(id)
            .await
            .unwrap();
        assert_eq!(system.name, "Amiga 500");
    }

    #[async_std::test]
    async fn delete_system_removes_record() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = SystemService::new(Arc::clone(&repo_manager));
        let id = service.add_system("Commodore 64").await.unwrap();
        service.delete_system(id).await.unwrap();
        let result = repo_manager
            .get_system_repository()
            .get_system(id)
            .await;
        assert!(result.is_err());
    }

    #[async_std::test]
    async fn delete_system_in_use_returns_error() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = SystemService::new(Arc::clone(&repo_manager));
        let system_id = service.add_system("Commodore 64").await.unwrap();
        repo_manager
            .get_release_repository()
            .add_release_full("Test Release", &[], &[], &[system_id])
            .await
            .unwrap();
        let result = service.delete_system(system_id).await;
        assert!(result.is_err());
    }
}
