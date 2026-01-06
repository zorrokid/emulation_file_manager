use std::sync::Arc;

use core_types::{FileType, Sha1Checksum};
use sqlx::{Pool, QueryBuilder, Row, Sqlite, prelude::FromRow, sqlite::SqliteRow};

use crate::{database_error::Error, models::FileInfo};

#[derive(Debug)]
pub struct FileInfoRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl FromRow<'_, SqliteRow> for FileInfo {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let file_type_int: u8 = row.try_get("file_type")?;
        let file_type: FileType =
            FileType::from_db_int(file_type_int).expect("Invalid file type in DB");
        let sha1_checksum: Vec<u8> = row.try_get("sha1_checksum")?;
        let sha1_checksum: Sha1Checksum = sha1_checksum
            .try_into()
            .expect("Invalid SHA1 checksum length in DB");
        Ok(Self {
            id: row.try_get("id")?,
            file_type,
            sha1_checksum,
            file_size: row.try_get("file_size")?,
            archive_file_name: row.try_get("archive_file_name")?,
        })
    }
}

impl FileInfoRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn add_file_info(
        &self,
        sha1_checksum: &Sha1Checksum,
        file_size: i64,
        archive_file_name: &str,
        file_type: FileType,
    ) -> Result<i64, Error> {
        let file_type = file_type.to_db_int();
        let sha1_checksum = sha1_checksum.to_vec();
        let result = sqlx::query!(
            "INSERT INTO file_info (
                sha1_checksum, 
                file_size, 
                archive_file_name,
                file_type
                ) VALUES (?, ?, ?, ?)",
            sha1_checksum,
            file_size,
            archive_file_name,
            file_type
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_file_info(&self, id: i64) -> Result<FileInfo, Error> {
        let result = sqlx::query_as::<_, FileInfo>(
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type
             FROM file_info WHERE id = ?",
        )
        .bind(id)
        .fetch_one(&*self.pool)
        .await?;
        Ok(result)
    }

    pub async fn get_file_infos_by_sha1_checksums(
        &self,
        checksums: &[Sha1Checksum],
        file_type: FileType,
    ) -> Result<Vec<FileInfo>, Error> {
        let mut query_builder = QueryBuilder::<Sqlite>::new(
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type
             FROM file_info WHERE file_type = ",
        );
        query_builder.push_bind(file_type.to_db_int());
        query_builder.push(" AND sha1_checksum IN (");
        let mut separated = query_builder.separated(", ");
        for checksum in checksums {
            separated.push_bind(checksum.to_vec());
        }
        separated.push_unseparated(")");
        let query = query_builder.build_query_as::<FileInfo>();
        let file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_infos)
    }

    pub async fn get_file_infos_by_file_set(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<FileInfo>, Error> {
        let query = sqlx::query_as::<_, FileInfo>(
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type
             FROM file_info fi
             JOIN file_set_file_info fsfi ON fi.id = fsfi.file_info_id
             WHERE fsfi.file_set_id = ?",
        )
        .bind(file_set_id);
        let file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_infos)
    }

    pub async fn get_file_infos_without_sync_log(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FileInfo>, Error> {
        let query = sqlx::query_as::<_, FileInfo>(
            "SELECT fi.id, fi.sha1_checksum, fi.file_size, fi.archive_file_name, fi.file_type
             FROM file_info fi
             LEFT JOIN file_sync_log fsl ON fi.id = fsl.file_info_id
             WHERE fsl.file_info_id IS NULL 
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset);
        let file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_infos)
    }

    pub async fn delete_file_info(&self, id: i64) -> Result<(), Error> {
        let query = sqlx::query("DELETE FROM file_info WHERE id = ?").bind(id);
        query.execute(&*self.pool).await?;
        Ok(())
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
        let checksum_1 = Sha1Checksum::from([0; 20]);
        let checksum_2 = Sha1Checksum::from([1; 20]);
        let file_type = FileType::Rom.to_db_int();

        query(
            "INSERT INTO file_info (
                sha1_checksum, 
                file_size, 
                archive_file_name,
                file_type
                ) VALUES (?, ?, ?, ?)",
        )
        .bind(checksum_1.to_vec())
        .bind(1234)
        .bind("test_archive_name_1")
        .bind(file_type)
        .execute(&*pool)
        .await
        .unwrap();

        query(
            "INSERT INTO file_info (
                sha1_checksum,
                file_size,
                archive_file_name,
                file_type
                ) VALUES (?, ?, ?, ?)",
        )
        .bind(checksum_2.to_vec())
        .bind(5678)
        .bind("test_archive_name_2")
        .bind(file_type)
        .execute(&*pool)
        .await
        .unwrap();

        let checksums: Vec<Sha1Checksum> = vec![checksum_1, checksum_2];
        let file_infos = file_info_repository
            .get_file_infos_by_sha1_checksums(&checksums, FileType::Rom)
            .await
            .unwrap();

        assert_eq!(file_infos.len(), 2);
    }

    #[async_std::test]
    async fn test_file_infos_get_by_file_set() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let file_info_repository = FileInfoRepository::new(pool.clone());
        let checksum_1: Sha1Checksum = [0u8; 20];
        let checksum_1_bytes = checksum_1.to_vec();
        let file_type = FileType::Rom.to_db_int();

        let result = query!(
            "INSERT INTO file_info (
                sha1_checksum, 
                file_size,
                archive_file_name,
                file_type
                ) VALUES (?, ?, ?, ?)",
            checksum_1_bytes,
            1234,
            "test_archive_name_1",
            file_type
        )
        .execute(&*pool)
        .await
        .unwrap();

        let file_info_id = result.last_insert_rowid();
        let checksum_2: Sha1Checksum = [1u8; 20];
        let checksum_2_bytes = checksum_2.to_vec();

        let result = query!(
            "INSERT INTO file_info (
                sha1_checksum, 
                file_size,
                archive_file_name,
                file_type
                ) VALUES (?, ?, ?, ?)",
            checksum_2_bytes,
            5678,
            "test_archive_name_2",
            file_type
        )
        .execute(&*pool)
        .await
        .unwrap();

        let file_info_id_2 = result.last_insert_rowid();

        let result = query!(
            "INSERT INTO file_set (file_name, file_type, name) 
             VALUES (?, ?, ?)",
            "test_file_set",
            file_type,
            "test_file_set_name"
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
        assert_eq!(file_infos[0].sha1_checksum, checksum_1);
        assert_eq!(file_infos[0].file_size, 1234);
        assert_eq!(file_infos[1].sha1_checksum, checksum_2);
        assert_eq!(file_infos[1].file_size, 5678);
    }
}
