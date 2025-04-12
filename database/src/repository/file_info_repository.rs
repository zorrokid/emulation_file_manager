use std::sync::Arc;

use sqlx::{Pool, QueryBuilder, Sqlite};

use crate::{database_error::DatabaseError, models::FileInfo};

pub struct FileInfoRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl FileInfoRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
    pub async fn get_file_infos_by_sha1_checksums(
        &self,
        checksums: Vec<String>,
    ) -> Result<Vec<FileInfo>, DatabaseError> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(
            "SELECT id, sha1_checksum, file_size 
             FROM file_info WHERE sha1_checksum IN (",
        );
        let mut separated = query_builder.separated(", ");
        for checksum in &checksums {
            separated.push_bind(checksum);
        }
        separated.push_unseparated(")");
        let query = query_builder.build_query_as::<FileInfo>();
        let file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_infos)
    }
}
#[cfg(test)]
mod tests {
    use crate::setup_test_db;

    use super::*;
    use sqlx::query;

    #[async_std::test]
    async fn test_file_infos_get_by_sha1_checksums() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let file_info_repository = FileInfoRepository::new(pool.clone());

        query("INSERT INTO file_info (sha1_checksum, file_size) VALUES (?, ?)")
            .bind("test_sha1_1")
            .bind(1234)
            .execute(&*pool)
            .await
            .unwrap();

        query("INSERT INTO file_info (sha1_checksum, file_size) VALUES (?, ?)")
            .bind("test_sha1_2")
            .bind(5678)
            .execute(&*pool)
            .await
            .unwrap();

        let checksums = vec!["test_sha1_1".to_string(), "test_sha1_2".to_string()];
        let file_infos = file_info_repository
            .get_file_infos_by_sha1_checksums(checksums)
            .await
            .unwrap();

        assert_eq!(file_infos.len(), 2);
    }
}
