use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::database_error::DatabaseError;

pub struct CoreMapping {
    pub id: i64,
    pub core_name: String,
}

pub struct SystemLibretroCoreMapping {
    pub id: i64,
    pub system_id: i64,
    pub system_name: String,
    pub core_name: String,
}

#[derive(Debug)]
pub struct SystemLibretroCoreRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl SystemLibretroCoreRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn add_mapping(&self, system_id: i64, core_name: &str) -> Result<i64, DatabaseError> {
        if core_name.is_empty() {
            return Err(DatabaseError::ValidationError(
                "core_name cannot be empty".to_string(),
            ));
        }
        let result = sqlx::query!(
            "INSERT INTO system_libretro_core (system_id, core_name) VALUES (?, ?)",
            system_id,
            core_name
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_mappings_for_system(
        &self,
        system_id: i64,
    ) -> Result<Vec<CoreMapping>, DatabaseError> {
        let mappings = sqlx::query_as!(
            CoreMapping,
            "SELECT id, core_name FROM system_libretro_core WHERE system_id = ?",
            system_id
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(mappings)
    }

    pub async fn get_mappings_for_core(
        &self,
        core_name: &str,
    ) -> Result<Vec<SystemLibretroCoreMapping>, DatabaseError> {
        let mappings = sqlx::query_as!(
            SystemLibretroCoreMapping,
            "SELECT slc.id, slc.system_id, s.name as system_name, slc.core_name
             FROM system_libretro_core slc
             JOIN system s ON s.id = slc.system_id
             WHERE slc.core_name = ?",
            core_name
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(mappings)
    }

    pub async fn remove_mapping(&self, id: i64) -> Result<(), DatabaseError> {
        sqlx::query!("DELETE FROM system_libretro_core WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }

    pub async fn remove_all_for_system(&self, system_id: i64) -> Result<(), DatabaseError> {
        sqlx::query!(
            "DELETE FROM system_libretro_core WHERE system_id = ?",
            system_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{repository::system_repository::SystemRepository, setup_test_db};

    async fn setup(pool: Arc<Pool<Sqlite>>) -> (SystemRepository, SystemLibretroCoreRepository) {
        let system_repo = SystemRepository::new(pool.clone());
        let core_repo = SystemLibretroCoreRepository::new(pool.clone());
        (system_repo, core_repo)
    }

    #[async_std::test]
    async fn test_add_and_get_mapping() {
        let pool = Arc::new(setup_test_db().await);
        let (system_repo, core_repo) = setup(pool).await;

        let system_id = system_repo.add_system("NES").await.unwrap();
        let mapping_id = core_repo
            .add_mapping(system_id, "fceumm_libretro")
            .await
            .unwrap();
        assert!(mapping_id > 0);

        let mappings = core_repo
            .get_mappings_for_system(system_id)
            .await
            .unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].id, mapping_id);
        assert_eq!(mappings[0].core_name, "fceumm_libretro");
    }

    #[async_std::test]
    async fn test_get_mappings_for_core() {
        let pool = Arc::new(setup_test_db().await);
        let (system_repo, core_repo) = setup(pool).await;

        let system_id = system_repo.add_system("NES").await.unwrap();
        core_repo
            .add_mapping(system_id, "fceumm_libretro")
            .await
            .unwrap();

        let mappings = core_repo
            .get_mappings_for_core("fceumm_libretro")
            .await
            .unwrap();
        assert_eq!(mappings.len(), 1);
        assert_eq!(mappings[0].system_id, system_id);
        assert_eq!(mappings[0].system_name, "NES");
        assert_eq!(mappings[0].core_name, "fceumm_libretro");
    }

    #[async_std::test]
    async fn test_remove_mapping() {
        let pool = Arc::new(setup_test_db().await);
        let (system_repo, core_repo) = setup(pool).await;

        let system_id = system_repo.add_system("NES").await.unwrap();
        let mapping_id = core_repo
            .add_mapping(system_id, "fceumm_libretro")
            .await
            .unwrap();

        core_repo.remove_mapping(mapping_id).await.unwrap();

        let mappings = core_repo
            .get_mappings_for_system(system_id)
            .await
            .unwrap();
        assert!(mappings.is_empty());
    }

    #[async_std::test]
    async fn test_duplicate_mapping_rejected() {
        let pool = Arc::new(setup_test_db().await);
        let (system_repo, core_repo) = setup(pool).await;

        let system_id = system_repo.add_system("NES").await.unwrap();
        core_repo
            .add_mapping(system_id, "fceumm_libretro")
            .await
            .unwrap();

        let result = core_repo.add_mapping(system_id, "fceumm_libretro").await;
        assert!(result.is_err());
    }

    #[async_std::test]
    async fn test_empty_core_name_rejected() {
        let pool = Arc::new(setup_test_db().await);
        let (system_repo, core_repo) = setup(pool).await;

        let system_id = system_repo.add_system("NES").await.unwrap();
        let result = core_repo.add_mapping(system_id, "").await;
        assert!(matches!(result, Err(DatabaseError::ValidationError(_))));
    }

    #[async_std::test]
    async fn test_cascade_delete_on_system_removal() {
        let pool = Arc::new(setup_test_db().await);
        let (system_repo, core_repo) = setup(pool.clone()).await;

        let system_id = system_repo.add_system("NES").await.unwrap();
        core_repo
            .add_mapping(system_id, "fceumm_libretro")
            .await
            .unwrap();

        system_repo.delete_system(system_id).await.unwrap();

        let mappings = core_repo
            .get_mappings_for_system(system_id)
            .await
            .unwrap();
        assert!(mappings.is_empty());
    }

    #[async_std::test]
    async fn test_remove_all_for_system() {
        let pool = Arc::new(setup_test_db().await);
        let (system_repo, core_repo) = setup(pool).await;

        let system_id = system_repo.add_system("NES").await.unwrap();
        core_repo
            .add_mapping(system_id, "fceumm_libretro")
            .await
            .unwrap();
        core_repo
            .add_mapping(system_id, "snes9x_libretro")
            .await
            .unwrap();

        core_repo.remove_all_for_system(system_id).await.unwrap();

        let mappings = core_repo
            .get_mappings_for_system(system_id)
            .await
            .unwrap();
        assert!(mappings.is_empty());
    }
}
