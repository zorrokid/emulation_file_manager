use std::sync::Arc;

use core_types::{ArgumentType, DocumentType};
use database::repository_manager::RepositoryManager;

use crate::error::Error;

#[derive(Debug)]
pub struct DocumentViewerService {
    repository_manager: Arc<RepositoryManager>,
}

impl DocumentViewerService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn add_document_viewer(
        &self,
        name: &str,
        executable: &str,
        arguments: &[ArgumentType],
        document_type: &DocumentType,
        cleanup_temp_files: bool,
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_document_viewer_repository()
            .add_document_viewer(
                &name.to_string(),
                &executable.to_string(),
                arguments,
                document_type,
                cleanup_temp_files,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn update_document_viewer(
        &self,
        id: i64,
        name: &str,
        executable: &str,
        arguments: &[ArgumentType],
        document_type: &DocumentType,
        cleanup_temp_files: bool,
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_document_viewer_repository()
            .update_document_viewer(
                id,
                &name.to_string(),
                &executable.to_string(),
                &arguments.to_vec(),
                document_type,
                cleanup_temp_files,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn delete_document_viewer(&self, id: i64) -> Result<i64, Error> {
        self.repository_manager
            .get_document_viewer_repository()
            .delete(id)
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use core_types::{ArgumentType, DocumentType};

    use super::*;

    #[async_std::test]
    async fn add_document_viewer_returns_positive_id() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = DocumentViewerService::new(repo_manager);
        let id = service
            .add_document_viewer("Evince", "/usr/bin/evince", &[], &DocumentType::Pdf, false)
            .await
            .unwrap();
        assert!(id > 0);
    }

    #[async_std::test]
    async fn add_document_viewer_persists_all_fields() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = DocumentViewerService::new(Arc::clone(&repo_manager));
        let id = service
            .add_document_viewer("Evince", "/usr/bin/evince", &[], &DocumentType::Pdf, true)
            .await
            .unwrap();
        let viewers = repo_manager
            .get_document_viewer_repository()
            .get_document_viewers()
            .await
            .unwrap();
        let viewer = viewers.iter().find(|v| v.id == id).unwrap();
        assert_eq!(viewer.name, "Evince");
        assert_eq!(viewer.executable, "/usr/bin/evince");
        assert_eq!(viewer.document_type, DocumentType::Pdf);
        assert!(viewer.cleanup_temp_files);
    }

    #[async_std::test]
    async fn add_document_viewer_with_arguments_round_trips() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = DocumentViewerService::new(Arc::clone(&repo_manager));
        let args = vec![ArgumentType::Flag { name: "--fullscreen".to_string() }];
        let id = service
            .add_document_viewer("Evince", "/usr/bin/evince", &args, &DocumentType::Pdf, false)
            .await
            .unwrap();
        let viewers = repo_manager
            .get_document_viewer_repository()
            .get_document_viewers()
            .await
            .unwrap();
        let viewer = viewers.iter().find(|v| v.id == id).unwrap();
        let stored: Vec<ArgumentType> = serde_json::from_str(&viewer.arguments).unwrap();
        assert_eq!(stored.len(), 1);
    }

    #[async_std::test]
    async fn update_document_viewer_changes_fields() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = DocumentViewerService::new(Arc::clone(&repo_manager));
        let id = service
            .add_document_viewer("Evince", "/usr/bin/evince", &[], &DocumentType::Pdf, false)
            .await
            .unwrap();
        service
            .update_document_viewer(
                id,
                "Okular",
                "/usr/bin/okular",
                &[],
                &DocumentType::Pdf,
                true,
            )
            .await
            .unwrap();
        let viewers = repo_manager
            .get_document_viewer_repository()
            .get_document_viewers()
            .await
            .unwrap();
        let viewer = viewers.iter().find(|v| v.id == id).unwrap();
        assert_eq!(viewer.name, "Okular");
        assert_eq!(viewer.executable, "/usr/bin/okular");
        assert!(viewer.cleanup_temp_files);
    }

    #[async_std::test]
    async fn delete_document_viewer_removes_record() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = DocumentViewerService::new(Arc::clone(&repo_manager));
        let id = service
            .add_document_viewer("Evince", "/usr/bin/evince", &[], &DocumentType::Pdf, false)
            .await
            .unwrap();
        service.delete_document_viewer(id).await.unwrap();
        let viewers = repo_manager
            .get_document_viewer_repository()
            .get_document_viewers()
            .await
            .unwrap();
        assert!(!viewers.iter().any(|v| v.id == id));
    }
}
