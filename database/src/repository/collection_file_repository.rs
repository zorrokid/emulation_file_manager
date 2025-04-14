use std::sync::Arc;

use sqlx::{sqlite::SqliteRow, FromRow, Pool, Sqlite};

use crate::models::{CollectionFile, FileType};

pub struct CollectionFileRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl CollectionFileRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}

impl FromRow<'_, SqliteRow> for CollectionFile {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        // TODO - handle try_into error instead of unwrap
        let file_type: FileType = row.try_get::<i64, _>("file_type")?.try_into().unwrap();
        Ok(Self {
            id: row.try_get("id")?,
            file_name: row.try_get("file_name")?,
            file_type,
        })
    }
}

impl CollectionFileReadRepository for CollectionFileRepository {
    async fn get_collection_files_for_release(
        &self,
        release_id: i64,
    ) -> Result<Vec<CollectionFile>, DatabaseError> {
        let collection_files = sqlx::query_as(
            "SELECT c.id, c.original_file_name, c.is_archive, c.archive_type, c.file_type 
             FROM collection_file c 
             INNER JOIN release_collection_file rcf
             ON c.id = rcf.collection_file_id
             WHERE rcf.release_id = ?",
        )
        .bind(release_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(collection_files)
    }

    async fn is_collection_file_in_release(
        &self,
        collection_file_id: i64,
    ) -> Result<bool, DatabaseError> {
        let count = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM release_collection_file 
             WHERE collection_file_id = ?",
            collection_file_id
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(count > 0)
    }

    async fn get_collection_files(
        &self,
        ids: Vec<i64>,
    ) -> Result<Vec<CollectionFile>, DatabaseError> {
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<&str>>().join(",");
        let query = format!(
            "SELECT id, original_file_name, archive_type, file_type 
             FROM collection_file 
             WHERE id IN ({})",
            placeholders
        );

        let mut query_builder = sqlx::query_as::<Sqlite, CollectionFile>(&query);
        for id in ids {
            query_builder = query_builder.bind(id);
        }

        let collection_files = query_builder.fetch_all(&*self.pool).await?;
        Ok(collection_files)
    }
}

impl CollectionFileWriteRepository for CollectionFileRepository {
    async fn add_collection_file(
        &self,
        original_file_name: String,
        collection_file_name: String,
        archive_type: ArchiveType,
        file_type: FileType,
        files: Vec<PickedFileInfo>,
    ) -> Result<i64, DatabaseError> {
        let archive_type = archive_type as i64;
        let file_type = file_type as i64;

        let mut transaction = self.pool.begin().await?;

        let result = sqlx::query!(
            "INSERT INTO collection_file (
                original_file_name, 
                collection_file_name,
                archive_type, 
                file_type) 
             VALUES (?, ?, ?, ?)",
            original_file_name,
            collection_file_name,
            archive_type,
            file_type
        )
        .execute(&mut *transaction)
        .await?;
        let collection_file_id = result.last_insert_rowid();

        for file in files {
            sqlx::query!(
                "INSERT INTO file_info (file_name, sha1_checksum, file_size) VALUES (?, ?, ?)",
                file.original_file_name,
                file.sha1_checksum,
                file.file_size
            )
            .execute(&mut *transaction)
            .await?;

            let file_info_id = result.last_insert_rowid();

            sqlx::query!(
                "INSERT INTO collection_file_file_info (collection_file_id, file_info_id) 
                 VALUES (?, ?)",
                collection_file_id,
                file_info_id
            )
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;

        Ok(collection_file_id)
    }

    async fn delete_collection_file(&self, id: i64) -> Result<i64, DatabaseError> {
        let is_in_use = sqlx::query_scalar!(
            "SELECT COUNT(*) 
             FROM release_collection_file 
             WHERE collection_file_id = ?",
            id
        )
        .fetch_one(&*self.pool)
        .await?;

        if is_in_use > 0 {
            return Err(DatabaseError::InUse);
        }

        // TODO check also if file_info is linked to other collection file

        let mut transaction = self.pool.begin().await?;

        sqlx::query!(
            "DELETE FROM collection_file_file_info WHERE collection_file_id = ?",
            id
        )
        .execute(&mut *transaction)
        .await?;

        sqlx::query!("DELETE FROM collection_file WHERE id = ?", id)
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use crate::database::database_with_sqlx::get_memory_db_pool;

    use super::*;
    use sqlx::query;

    #[async_std::test]
    async fn test_get_collection_files_for_release() {
        let pool = get_memory_db_pool().await.unwrap();
        let collection_file = CollectionFile {
            id: 1,
            original_file_name: "test.zip".to_string(),
            collection_file_name: "1".to_string(),
            archive_type: ArchiveType::None,
            file_type: FileType::Rom,
        };
        let file_type = collection_file.file_type as i64;
        let archive_type = collection_file.archive_type as i64;
        query!(
            "INSERT INTO collection_file (
                original_file_name, 
                collection_file_name, 
                archive_type, 
                file_type
            ) VALUES (?, ?, ?, ?)",
            collection_file.original_file_name,
            collection_file.collection_file_name,
            archive_type,
            file_type
        )
        .execute(&pool)
        .await
        .unwrap();
        query!(
            "INSERT INTO release_collection_file (release_id, collection_file_id) VALUES (?, ?)",
            1,
            1
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = CollectionFileRepository {
            pool: Arc::new(pool),
        }
        .get_collection_files_for_release(1)
        .await
        .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], collection_file);
    }

    #[async_std::test]
    async fn test_is_collection_file_in_release() {
        let pool = get_memory_db_pool().await.unwrap();
        query!(
            "INSERT INTO collection_file (
                original_file_name, 
                collection_file_name, 
                archive_type, 
                file_type
            ) VALUES (?, ?, ?, ?)",
            "test.zip",
            "1",
            ArchiveType::None as i64,
            CollectionFileType::Rom as i64,
        )
        .execute(&pool)
        .await
        .unwrap();
        query!(
            "INSERT INTO release_collection_file (release_id, collection_file_id) VALUES (?, ?)",
            1,
            1
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = CollectionFileRepository {
            pool: Arc::new(pool),
        }
        .is_collection_file_in_release(1)
        .await
        .unwrap();
        assert_eq!(result, true);
    }

    #[async_std::test]
    async fn test_add_collection_file() {
        let pool = get_memory_db_pool().await.unwrap();
        let collection_file = CollectionFile {
            id: 1,
            original_file_name: "test.zip".to_string(),
            collection_file_name: "1".to_string(),
            archive_type: ArchiveType::None,
            file_type: FileType::Rom,
        };
        let result = CollectionFileRepository {
            pool: Arc::new(pool),
        }
        .add_collection_file(
            collection_file.original_file_name,
            collection_file.collection_file_name,
            collection_file.archive_type,
            collection_file.file_type,
        )
        .await
        .unwrap();
        assert_eq!(result, collection_file.id);
    }

    #[async_std::test]
    async fn test_delete_collection_file() {
        let pool = get_memory_db_pool().await.unwrap();
        let collection_file = CollectionFile {
            id: 1,
            original_file_name: "test.zip".to_string(),
            collection_file_name: "1".to_string(),
            archive_type: ArchiveType::None,
            file_type: FileType::Rom,
        };
        let file_type = collection_file.file_type as i64;
        let archive_type = collection_file.archive_type as i64;
        query!(
            "INSERT INTO collection_file (
                original_file_name, 
                collection_file_name,
                archive_type, 
                file_type
            ) VALUES (?, ?, ?, ?)",
            collection_file.original_file_name,
            collection_file.collection_file_name,
            archive_type,
            file_type
        )
        .execute(&pool)
        .await
        .unwrap();

        let repository = CollectionFileRepository {
            pool: Arc::new(pool),
        };

        let result = repository
            .delete_collection_file(collection_file.id)
            .await
            .unwrap();
        assert_eq!(result, ());

        let result = repository
            .get_collection_files_for_release(1)
            .await
            .unwrap();
        assert_eq!(result.len(), 0);
    }

    #[async_std::test]
    async fn test_delete_collection_file_in_use() {
        let pool = get_memory_db_pool().await.unwrap();
        let collection_file = CollectionFile {
            id: 1,
            original_file_name: "test.zip".to_string(),
            collection_file_name: "1".to_string(),
            archive_type: ArchiveType::Zip,
            file_type: FileType::Rom,
        };
        let file_type = collection_file.file_type as i64;
        let archive_type = collection_file.archive_type as i64;
        query!(
            "INSERT INTO collection_file (
                original_file_name, 
                collection_file_name,
                archive_type, 
                file_type
            ) VALUES (?, ?, ?, ?)",
            collection_file.original_file_name,
            collection_file.collection_file_name,
            archive_type,
            file_type
        )
        .execute(&pool)
        .await
        .unwrap();
        query!(
            "INSERT INTO release_collection_file (release_id, collection_file_id) VALUES (?, ?)",
            1,
            1
        )
        .execute(&pool)
        .await
        .unwrap();

        let result = CollectionFileRepository {
            pool: Arc::new(pool),
        }
        .delete_collection_file(collection_file.id)
        .await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), DatabaseError::InUse);
    }
}
