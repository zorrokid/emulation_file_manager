use std::sync::Arc;

use sqlx::{sqlite::SqliteRow, FromRow, Pool, Row, Sqlite};

use crate::{
    database_error::DatabaseError,
    models::{FileSet, FileType, PickedFileInfo},
};

pub struct FileSetRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl FileSetRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}

impl FromRow<'_, SqliteRow> for FileSet {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let file_type: FileType = row.try_get::<i64, _>("file_type")?.try_into()?;
        Ok(Self {
            id: row.try_get("id")?,
            file_name: row.try_get("file_name")?,
            file_type,
        })
    }
}

impl FileSetRepository {
    async fn get_file_sets_for_release(
        &self,
        release_id: i64,
    ) -> Result<Vec<FileSet>, DatabaseError> {
        let collection_files = sqlx::query_as(
            "SELECT c.id, c.file_name, c.file_type 
             FROM file_set c 
             INNER JOIN release_file_set rcf
             ON c.id = rcf.file_set_id
             WHERE rcf.release_id = ?",
        )
        .bind(release_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(collection_files)
    }

    async fn is_file_set_in_release(&self, file_set_id: i64) -> Result<bool, DatabaseError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM release_file_set
             WHERE file_set_id = ?",
            file_set_id
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(count > 0)
    }

    pub async fn get_file_sets(&self, ids: Vec<i64>) -> Result<Vec<FileSet>, DatabaseError> {
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!(
            "SELECT id, file_name,  file_type 
             FROM file_set
             WHERE id IN ({})",
            placeholders
        );

        let mut query_builder = sqlx::query_as::<Sqlite, FileSet>(&query);
        for id in ids {
            query_builder = query_builder.bind(id);
        }

        let file_sets = query_builder.fetch_all(&*self.pool).await?;
        Ok(file_sets)
    }

    pub async fn add_file_set(
        &self,
        file_name: &str,
        file_type: &FileType,
        files: &Vec<PickedFileInfo>,
    ) -> Result<i64, DatabaseError> {
        let file_type = *file_type as i64;

        let mut transaction = self.pool.begin().await?;

        let result = sqlx::query!(
            "INSERT INTO file_set(
                file_name, 
                file_type) 
             VALUES (?, ?)",
            file_name,
            file_type
        )
        .execute(&mut *transaction)
        .await?;
        let collection_file_id = result.last_insert_rowid();

        for file in files {
            let file_info_result = sqlx::query!(
                "INSERT INTO file_info (sha1_checksum, file_size) VALUES (?, ?)",
                file.sha1_checksum,
                file.file_size
            )
            .execute(&mut *transaction)
            .await?;

            let file_info_id = file_info_result.last_insert_rowid();

            sqlx::query!(
                "INSERT INTO file_set_file_info (file_set_id, file_info_id, file_name) 
                 VALUES (?, ?, ?)",
                collection_file_id,
                file_info_id,
                file.file_name
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(collection_file_id)
    }

    async fn delete_file_set(&self, id: i64) -> Result<i64, DatabaseError> {
        let is_in_use = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM release_file_set
             WHERE file_set_id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;

        if is_in_use > 0 {
            return Err(DatabaseError::InUse);
        }

        // TODO check also if file_info is linked to other file set(s)

        let mut transaction = self.pool.begin().await?;

        sqlx::query!("DELETE FROM file_set_file_info WHERE file_set_id = ?", id)
            .execute(&mut *transaction)
            .await?;

        sqlx::query!("DELETE FROM file_set WHERE id = ?", id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use crate::setup_test_db;

    use super::*;
    use sqlx::{query, query_scalar};

    #[async_std::test]
    async fn test_get_file_sets_for_release() {
        let pool = setup_test_db().await;
        let collection_file = FileSet {
            id: 1,
            file_name: "test.zip".to_string(),
            file_type: FileType::Rom,
        };
        let file_type = collection_file.file_type as i64;

        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type
            ) VALUES (?, ?)",
            collection_file.file_name,
            file_type
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_result = query!(
            "INSERT INTO release (
                name
            ) VALUES (?)",
            "test",
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_id = release_result.last_insert_rowid();
        let file_set_id = result.last_insert_rowid();

        query!(
            "INSERT INTO release_file_set(release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = FileSetRepository {
            pool: Arc::new(pool),
        }
        .get_file_sets_for_release(release_id)
        .await
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, file_set_id);
    }

    #[async_std::test]
    async fn test_is_file_set_in_release() {
        let pool = setup_test_db().await;
        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type
            ) VALUES (?, ?)",
            "test",
            FileType::Rom as i64,
        )
        .execute(&pool)
        .await
        .unwrap();
        let release_result = query!(
            "INSERT INTO release (
                name
            ) VALUES (?)",
            "test",
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_id = release_result.last_insert_rowid();
        let file_set_id = result.last_insert_rowid();

        query!(
            "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = FileSetRepository {
            pool: Arc::new(pool),
        }
        .is_file_set_in_release(release_id)
        .await
        .unwrap();
        assert!(result);
    }

    #[async_std::test]
    async fn test_add_file_set() {
        let pool = Arc::new(setup_test_db().await);
        let file_name = "test file".to_string();
        let file_type = FileType::Rom;
        let files = vec![
            PickedFileInfo {
                sha1_checksum: "test".to_string(),
                file_size: 123,
                file_name: "test".to_string(),
            },
            PickedFileInfo {
                sha1_checksum: "test2".to_string(),
                file_size: 123,
                file_name: "test2".to_string(),
            },
        ];
        let file_set_id = FileSetRepository { pool: pool.clone() }
            .add_file_set(&file_name, &file_type, &files)
            .await
            .unwrap();

        let files_for_file_set = query_scalar!(
            "SELECT COUNT(*) 
             FROM file_set_file_info 
             WHERE file_set_id = ?",
            file_set_id
        )
        .fetch_one(&*pool)
        .await
        .unwrap();

        assert_eq!(files_for_file_set, 2);
    }

    #[async_std::test]
    async fn test_delete_file_set() {
        let pool = setup_test_db().await;
        let file_name = "test file".to_string();
        let file_type = FileType::Rom as i64;
        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type
            ) VALUES (?, ?)",
            file_name,
            file_type
        )
        .execute(&pool)
        .await
        .unwrap();

        let file_set_id = result.last_insert_rowid();

        let repository = FileSetRepository {
            pool: Arc::new(pool),
        };

        let result = repository.delete_file_set(file_set_id).await.unwrap();
        assert_eq!(result, file_set_id);
        let result = repository.get_file_sets(vec![file_set_id]).await.unwrap();
        assert_eq!(result.len(), 0);
    }

    #[async_std::test]
    async fn test_delete_file_set_in_use() {
        let pool = setup_test_db().await;
        let file_name = "test file".to_string();
        let file_type = FileType::Rom as i64;
        let result = query!(
            "INSERT INTO file_set (
                file_name,
                file_type
            ) VALUES (?, ?)",
            file_name,
            file_type
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_result = query!(
            "INSERT INTO release (
                name
            ) VALUES (?)",
            "test",
        )
        .execute(&pool)
        .await
        .unwrap();

        let release_id = release_result.last_insert_rowid();
        let file_set_id = result.last_insert_rowid();

        query!(
            "INSERT INTO release_file_set (release_id, file_set_id) VALUES (?, ?)",
            release_id,
            file_set_id
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = FileSetRepository {
            pool: Arc::new(pool),
        }
        .delete_file_set(file_set_id)
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DatabaseError::InUse);
    }
}
