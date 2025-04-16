use std::sync::Arc;

use sqlx::{Pool, QueryBuilder, Sqlite};

use crate::{database_error::DatabaseError, models::FileInfo};

#[derive(Debug)]
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

    pub async fn get_file_infos_by_file_set(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<FileInfo>, DatabaseError> {
        let query = sqlx::query_as::<_, FileInfo>(
            "SELECT id, sha1_checksum, file_size 
             FROM file_info fi
             JOIN file_set_file_info fsfi ON fi.id = fsfi.file_info_id
             WHERE fsfi.file_set_id = ?",
        )
        .bind(file_set_id);
        let file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_infos)
    }
}
#[cfg(test)]
mod tests {
    use crate::{models::FileType, setup_test_db};

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

    #[async_std::test]
    async fn test_file_infos_get_by_file_set() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let file_info_repository = FileInfoRepository::new(pool.clone());

        let result = query!(
            "INSERT INTO file_info (
                sha1_checksum, 
                file_size
                ) VALUES (?, ?)",
            "test_sha1_1",
            1234
        )
        .execute(&*pool)
        .await
        .unwrap();

        let file_info_id = result.last_insert_rowid();

        let result = query!(
            "INSERT INTO file_info (
                sha1_checksum, 
                file_size
                ) VALUES (?, ?)",
            "test_sha1_2",
            5678
        )
        .execute(&*pool)
        .await
        .unwrap();

        let file_info_id_2 = result.last_insert_rowid();

        let result = query!(
            "INSERT INTO file_set (file_name, file_type) 
             VALUES (?, ?)",
            "test_file_set",
            FileType::Rom as i32,
        )
        .execute(&*pool)
        .await
        .unwrap();

        let file_set_id = result.last_insert_rowid();

        query!(
            "INSERT INTO file_set_file_info (
                file_set_id, 
                file_info_id,
                file_name
                ) VALUES (?, ?, ?)",
            file_set_id,
            file_info_id,
            "test_file_name_1"
        )
        .execute(&*pool)
        .await
        .unwrap();

        query!(
            "INSERT INTO file_set_file_info (
                file_set_id, 
                file_info_id,
                file_name
                ) VALUES (?, ?, ?)",
            file_set_id,
            file_info_id_2,
            "test_file_name_1"
        )
        .execute(&*pool)
        .await
        .unwrap();

        let file_infos = file_info_repository
            .get_file_infos_by_file_set(file_set_id)
            .await
            .unwrap();

        assert_eq!(file_infos.len(), 2);
        assert_eq!(file_infos[0].sha1_checksum, "test_sha1_1");
        assert_eq!(file_infos[0].file_size, 1234);
        assert_eq!(file_infos[1].sha1_checksum, "test_sha1_2");
        assert_eq!(file_infos[1].file_size, 5678);
    }
}
