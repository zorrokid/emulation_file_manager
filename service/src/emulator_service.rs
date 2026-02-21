use std::sync::Arc;

use core_types::ArgumentType;
use database::repository_manager::RepositoryManager;

use crate::error::Error;

#[derive(Debug)]
pub struct EmulatorService {
    repository_manager: Arc<RepositoryManager>,
}

impl EmulatorService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn add_emulator(
        &self,
        name: &str,
        executable: &str,
        extract_files: bool,
        arguments: &[ArgumentType],
        system_id: i64,
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_emulator_repository()
            .add_emulator(
                &name.to_string(),
                &executable.to_string(),
                extract_files,
                arguments,
                system_id,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn update_emulator(
        &self,
        id: i64,
        name: &str,
        executable: &str,
        extract_files: bool,
        arguments: &[ArgumentType],
        system_id: i64,
    ) -> Result<i64, Error> {
        self.repository_manager
            .get_emulator_repository()
            .update_emulator(
                id,
                &name.to_string(),
                &executable.to_string(),
                extract_files,
                &arguments.to_vec(),
                system_id,
            )
            .await
            .map_err(Into::into)
    }

    pub async fn delete_emulator(&self, id: i64) -> Result<i64, Error> {
        self.repository_manager
            .get_emulator_repository()
            .delete_emulator(id)
            .await
            .map_err(|e| Error::DbError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use core_types::ArgumentType;

    use super::*;

    async fn add_system(repo_manager: &Arc<RepositoryManager>) -> i64 {
        repo_manager
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap()
    }

    #[async_std::test]
    async fn add_emulator_returns_positive_id() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = EmulatorService::new(Arc::clone(&repo_manager));
        let system_id = add_system(&repo_manager).await;
        let id = service
            .add_emulator("Vice", "/usr/bin/vice", false, &[], system_id)
            .await
            .unwrap();
        assert!(id > 0);
    }

    #[async_std::test]
    async fn add_emulator_with_empty_arguments_persists() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = EmulatorService::new(Arc::clone(&repo_manager));
        let system_id = add_system(&repo_manager).await;
        let id = service
            .add_emulator("Vice", "/usr/bin/vice", false, &[], system_id)
            .await
            .unwrap();
        let emulator = repo_manager
            .get_emulator_repository()
            .get_emulator(id)
            .await
            .unwrap();
        assert_eq!(emulator.name, "Vice");
        assert_eq!(emulator.arguments, "[]");
    }

    #[async_std::test]
    async fn add_emulator_with_arguments_round_trips() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = EmulatorService::new(Arc::clone(&repo_manager));
        let system_id = add_system(&repo_manager).await;
        let args = vec![ArgumentType::Flag { name: "-fullscreen".to_string() }];
        let id = service
            .add_emulator("Vice", "/usr/bin/vice", false, &args, system_id)
            .await
            .unwrap();
        let emulator = repo_manager
            .get_emulator_repository()
            .get_emulator(id)
            .await
            .unwrap();
        let stored: Vec<ArgumentType> = serde_json::from_str(&emulator.arguments).unwrap();
        assert_eq!(stored.len(), 1);
    }

    #[async_std::test]
    async fn update_emulator_changes_name_and_executable() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = EmulatorService::new(Arc::clone(&repo_manager));
        let system_id = add_system(&repo_manager).await;
        let id = service
            .add_emulator("Vice", "/usr/bin/vice", false, &[], system_id)
            .await
            .unwrap();
        service
            .update_emulator(id, "VICE 3.7", "/opt/vice/bin/vice", true, &[], system_id)
            .await
            .unwrap();
        let emulator = repo_manager
            .get_emulator_repository()
            .get_emulator(id)
            .await
            .unwrap();
        assert_eq!(emulator.name, "VICE 3.7");
        assert_eq!(emulator.executable, "/opt/vice/bin/vice");
        assert!(emulator.extract_files);
    }

    #[async_std::test]
    async fn delete_emulator_removes_record() {
        let repo_manager = database::setup_test_repository_manager().await;
        let service = EmulatorService::new(Arc::clone(&repo_manager));
        let system_id = add_system(&repo_manager).await;
        let id = service
            .add_emulator("Vice", "/usr/bin/vice", false, &[], system_id)
            .await
            .unwrap();
        service.delete_emulator(id).await.unwrap();
        let result = repo_manager.get_emulator_repository().get_emulator(id).await;
        assert!(result.is_err());
    }
}
