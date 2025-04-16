use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::{database_error::DatabaseError, models::Franchise};

#[derive(Debug)]
pub struct FranchiseRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl FranchiseRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    async fn get_all_franchises(&self) -> Result<Vec<Franchise>, DatabaseError> {
        let franchises = sqlx::query_as!(Franchise, "SELECT id, name FROM franchise")
            .fetch_all(&*self.pool)
            .await?;
        Ok(franchises)
    }

    async fn add_franchise(&self, name: &str) -> Result<i64, DatabaseError> {
        let result = sqlx::query!("INSERT INTO franchise (name) VALUES (?)", name)
            .execute(&*self.pool)
            .await?;
        Ok(result.last_insert_rowid())
    }

    async fn update_franchise(&self, franchise: &Franchise) -> Result<i64, DatabaseError> {
        let result = sqlx::query!(
            "UPDATE franchise SET name = ? WHERE id = ?",
            franchise.name,
            franchise.id
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    async fn delete_franchise(&self, id: i64) -> Result<(), DatabaseError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM software_title WHERE franchise_id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;
        if count > 0 {
            return Err(DatabaseError::InUse);
        }
        sqlx::query!("DELETE FROM franchise WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup_test_db;

    #[async_std::test]
    async fn test_franchise_repository() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let franchise_repository = FranchiseRepository::new(pool.clone());

        // Add a new franchise
        let franchise_id = franchise_repository
            .add_franchise("Test Franchise")
            .await
            .unwrap();

        // Get all franchises
        let franchises = franchise_repository.get_all_franchises().await.unwrap();
        assert_eq!(franchises.len(), 1);
        assert_eq!(franchises[0].name, "Test Franchise");

        // Update the franchise
        let mut franchise = franchises[0].clone();
        franchise.name = "Updated Franchise".to_string();
        franchise_repository
            .update_franchise(&franchise)
            .await
            .unwrap();

        // Verify the update
        let updated_franchises = franchise_repository.get_all_franchises().await.unwrap();
        assert_eq!(updated_franchises[0].name, "Updated Franchise");

        // Delete the franchise
        franchise_repository
            .delete_franchise(franchise_id)
            .await
            .unwrap();

        // Verify deletion
        let franchises_after_deletion = franchise_repository.get_all_franchises().await.unwrap();
        assert_eq!(franchises_after_deletion.len(), 0);
    }
}
