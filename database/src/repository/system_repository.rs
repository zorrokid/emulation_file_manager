use std::sync::Arc;

use crate::{
    database_error::DatabaseError,
    models::{FileSet, FileType, PickedFileInfo, System},
};
use sqlx::{sqlite::SqliteRow, FromRow, Pool, Row, Sqlite};

pub struct SystemRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl SystemRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
    async fn get_system(&self, id: i64) -> Result<System, DatabaseError> {
        let system = sqlx::query_as!(
            System,
            "SELECT id, name 
             FROM system WHERE id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(system)
    }

    async fn get_systems(&self) -> Result<Vec<System>, DatabaseError> {
        let systems = sqlx::query_as!(System, "SELECT id, name FROM system")
            .fetch_all(&*self.pool)
            .await?;
        Ok(systems)
    }

    async fn is_system_in_use(&self, system_id: i64) -> Result<bool, DatabaseError> {
        let releases_count = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM release_system 
             WHERE system_id = ?",
            system_id
        )
        .fetch_one(&*self.pool)
        .await?;

        let emulators_count = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM emulator_system 
             WHERE system_id = ?",
            system_id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(releases_count > 0 || emulators_count > 0)
    }
}

#[cfg(test)]
mod tests {
    use crate::setup_test_db;

    use super::*;
    use sqlx::query;

    const TEST_SYSTEM_NAME: &str = "Commodore 64";

    #[async_std::test]
    async fn test_get_system() {
        let pool = setup_test_db().await;
        let id = insert_test_system(&pool, TEST_SYSTEM_NAME).await;
        let result = SystemRepository {
            pool: Arc::new(pool),
        }
        .get_system(id)
        .await
        .unwrap();
        assert_eq!(result.id, id);
        assert_eq!(result.name, TEST_SYSTEM_NAME);
    }

    #[async_std::test]
    async fn test_get_systems() {
        let pool = setup_test_db().await;
        let id = insert_test_system(&pool, TEST_SYSTEM_NAME).await;
        let result = SystemRepository {
            pool: Arc::new(pool),
        }
        .get_systems()
        .await
        .unwrap();
        let result = &result[0];
        assert_eq!(result.id, id);
        assert_eq!(result.name, TEST_SYSTEM_NAME);
    }

    async fn insert_test_system(pool: &Pool<Sqlite>, system_name: &str) -> i64 {
        let result = query!("INSERT INTO system (name) VALUES(?)", system_name)
            .execute(pool)
            .await
            .unwrap();
        result.last_insert_rowid()
    }
}
