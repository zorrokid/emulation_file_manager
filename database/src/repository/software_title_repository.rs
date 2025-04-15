use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::{database_error::DatabaseError, models::SoftwareTitle};

pub struct SoftwareTitleRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl SoftwareTitleRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    async fn get_software_title(&self, id: i64) -> Result<SoftwareTitle, DatabaseError> {
        let software_title = sqlx::query_as!(
            SoftwareTitle,
            "SELECT id, name, franchise_id FROM software_title WHERE id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;

        Ok(software_title)
    }

    async fn get_all_software_titles(&self) -> Result<Vec<SoftwareTitle>, DatabaseError> {
        let software_titles = sqlx::query_as!(
            SoftwareTitle,
            "SELECT id, name, franchise_id FROM software_title"
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(software_titles)
    }

    async fn add_software_title(
        &self,
        name: &str,
        franchise_id: Option<i64>,
    ) -> Result<i64, DatabaseError> {
        let result = sqlx::query!(
            "INSERT INTO software_title (name, franchise_id) VALUES (?, ?)",
            name,
            franchise_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    async fn update_software_title(
        &self,
        software_title: &SoftwareTitle,
    ) -> Result<i64, DatabaseError> {
        let result = sqlx::query!(
            "UPDATE software_title SET name = ?, franchise_id = ? WHERE id = ?",
            software_title.name,
            software_title.franchise_id,
            software_title.id
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    async fn delete_software_title(&self, id: i64) -> Result<i64, DatabaseError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM release_software_title WHERE software_title_id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;
        if count > 0 {
            return Err(DatabaseError::InUse);
        }
        sqlx::query!("DELETE FROM software_title WHERE id = ?", id)
            .execute(&*self.pool)
            .await?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup_test_db;

    #[async_std::test]
    async fn test_software_title_repository() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let software_title_repository = SoftwareTitleRepository::new(pool.clone());

        // Add a new software title
        let software_title_id = software_title_repository
            .add_software_title("Test Software Title", None)
            .await
            .unwrap();

        // Get the software title by ID
        let software_title = software_title_repository
            .get_software_title(software_title_id)
            .await
            .unwrap();
        assert_eq!(software_title.name, "Test Software Title");

        // Update the software title
        let updated_software_title = SoftwareTitle {
            id: software_title_id,
            name: "Updated Software Title".to_string(),
            franchise_id: None,
        };
        software_title_repository
            .update_software_title(&updated_software_title)
            .await
            .unwrap();

        // Get all software titles
        let all_software_titles = software_title_repository
            .get_all_software_titles()
            .await
            .unwrap();
        assert_eq!(all_software_titles.len(), 1);
    }
}
