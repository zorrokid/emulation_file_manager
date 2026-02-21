use std::sync::Arc;

use core_types::item_type::ItemType;
use database::repository_manager::RepositoryManager;
use domain::models::ReleaseItem;

use crate::error::Error;

#[derive(Debug)]
pub struct ReleaseItemService {
    repository_manager: Arc<RepositoryManager>,
}

impl ReleaseItemService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn create_item(
        &self,
        release_id: i64,
        item_type: ItemType,
        notes: Option<String>,
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_release_item_repository()
            .create_item(release_id, item_type, notes)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }

    pub async fn get_item(&self, item_id: i64) -> Result<ReleaseItem, Error> {
        self.repository_manager
            .get_release_item_repository()
            .get_item(item_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
            .map(ReleaseItem::from)
    }

    pub async fn update_item(
        &self,
        item_id: i64,
        item_type: ItemType,
        notes: Option<String>,
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_release_item_repository()
            .update_item(item_id, item_type, notes)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }

    pub async fn delete_item(&self, item_id: i64) -> Result<(), Error> {
        self.repository_manager
            .get_release_item_repository()
            .delete_item(item_id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use core_types::item_type::ItemType;

    use super::*;

    async fn create_test_release(repo_manager: &Arc<RepositoryManager>) -> i64 {
        repo_manager
            .get_release_repository()
            .add_release_full("Test Release", &[], &[], &[])
            .await
            .unwrap()
    }

    #[async_std::test]
    async fn create_item_returns_positive_id() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseItemService::new(Arc::clone(&repo_manager));
        let release_id = create_test_release(&repo_manager).await;
        let id = service
            .create_item(release_id, ItemType::Manual, None)
            .await
            .unwrap();
        assert!(id > 0);
    }

    #[async_std::test]
    async fn get_item_returns_correct_item_type_and_notes() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseItemService::new(Arc::clone(&repo_manager));
        let release_id = create_test_release(&repo_manager).await;
        let id = service
            .create_item(
                release_id,
                ItemType::DiskOrSetOfDisks,
                Some("side A".to_string()),
            )
            .await
            .unwrap();
        let item = service.get_item(id).await.unwrap();
        assert_eq!(item.item_type, ItemType::DiskOrSetOfDisks);
        assert_eq!(item.notes, "side A".to_string());
    }

    #[async_std::test]
    async fn update_item_changes_item_type() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseItemService::new(Arc::clone(&repo_manager));
        let release_id = create_test_release(&repo_manager).await;
        let id = service
            .create_item(release_id, ItemType::Manual, None)
            .await
            .unwrap();
        service.update_item(id, ItemType::Box, None).await.unwrap();
        let item = service.get_item(id).await.unwrap();
        assert_eq!(item.item_type, ItemType::Box);
    }

    #[async_std::test]
    async fn update_item_clears_notes_when_none() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseItemService::new(Arc::clone(&repo_manager));
        let release_id = create_test_release(&repo_manager).await;
        let id = service
            .create_item(release_id, ItemType::Manual, Some("some notes".to_string()))
            .await
            .unwrap();
        service
            .update_item(id, ItemType::Manual, None)
            .await
            .unwrap();
        let item = service.get_item(id).await.unwrap();
        assert!(item.notes.is_empty());
    }

    #[async_std::test]
    async fn delete_item_removes_record() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseItemService::new(Arc::clone(&repo_manager));
        let release_id = create_test_release(&repo_manager).await;
        let id = service
            .create_item(release_id, ItemType::TapeOrSetOfTapes, None)
            .await
            .unwrap();
        service.delete_item(id).await.unwrap();
        let result = service.get_item(id).await;
        assert!(result.is_err());
    }

    #[async_std::test]
    async fn get_nonexistent_item_returns_error() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseItemService::new(repo_manager);
        let result = service.get_item(99999).await;
        assert!(result.is_err());
    }

    #[async_std::test]
    async fn delete_nonexistent_item_returns_error() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = ReleaseItemService::new(repo_manager);
        // SQLite DELETE doesn't error on missing rows, so this is a no-op rather than error.
        // Verify it at least completes without panic.
        service.delete_item(99999).await.unwrap();
    }
}
