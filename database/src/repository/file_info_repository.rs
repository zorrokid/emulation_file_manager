use std::sync::Arc;

use core_types::{CloudSyncStatus, FileType, Sha1Checksum};
use sqlx::{Pool, QueryBuilder, Row, Sqlite, prelude::FromRow, sqlite::SqliteRow};

use crate::{database_error::Error, models::{CloudSyncableFileInfo, FileInfo}};

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
        let cloud_sync_status_int: u8 = row.try_get("cloud_sync_status")?;
        let cloud_sync_status = CloudSyncStatus::from_db_int(cloud_sync_status_int)
            .expect("Invalid cloud_sync_status in DB");
        Ok(Self {
            id: row.try_get("id")?,
            file_type,
            sha1_checksum,
            file_size: row.try_get("file_size")?,
            archive_file_name: row.try_get("archive_file_name")?,
            cloud_sync_status,
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
        archive_file_name: Option<&str>,
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
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
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
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
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
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
             FROM file_info fi
             JOIN file_set_file_info fsfi ON fi.id = fsfi.file_info_id
             WHERE fsfi.file_set_id = ?",
        )
        .bind(file_set_id);
        let file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_infos)
    }

    /// Returns available files ready for upload (NotSynced + archive_file_name IS NOT NULL), paginated.
    pub async fn get_files_pending_upload(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CloudSyncableFileInfo>, Error> {
        let status_int = CloudSyncStatus::NotSynced.to_db_int();
        let rows = sqlx::query_as::<_, FileInfo>(
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
             FROM file_info
             WHERE cloud_sync_status = ? AND archive_file_name IS NOT NULL
             LIMIT ? OFFSET ?",
        )
        .bind(status_int)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;
        rows.into_iter()
            .map(CloudSyncableFileInfo::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::DecodeError(
                "Unexpected null archive_file_name in pending upload query".into(),
            ))
    }

    /// Counts available files ready for upload (NotSynced + archive_file_name IS NOT NULL).
    pub async fn count_files_pending_upload(&self) -> Result<i64, Error> {
        let status_int = CloudSyncStatus::NotSynced.to_db_int();
        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM file_info WHERE cloud_sync_status = ? AND archive_file_name IS NOT NULL",
            status_int
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(row.count)
    }

    /// Returns tombstone file_infos awaiting cloud deletion (DeletionPending), paginated.
    /// All DeletionPending tombstones are processed regardless of archive_file_name.
    pub async fn get_files_pending_deletion(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FileInfo>, Error> {
        let status_int = CloudSyncStatus::DeletionPending.to_db_int();
        let query = sqlx::query_as::<_, FileInfo>(
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
             FROM file_info
             WHERE cloud_sync_status = ?
             LIMIT ? OFFSET ?",
        )
        .bind(status_int)
        .bind(limit)
        .bind(offset);
        let file_infos = query.fetch_all(&*self.pool).await?;
        Ok(file_infos)
    }

    /// Counts tombstone file_infos awaiting cloud deletion (DeletionPending).
    pub async fn count_files_pending_deletion(&self) -> Result<i64, Error> {
        let status_int = CloudSyncStatus::DeletionPending.to_db_int();
        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM file_info WHERE cloud_sync_status = ?",
            status_int
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(row.count)
    }

    /// Returns DeletionPending files that have an archive_file_name (i.e. were uploaded to cloud),
    /// paginated. These require a cloud delete operation before the record is removed.
    pub async fn get_cloud_files_pending_deletion(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<CloudSyncableFileInfo>, Error> {
        let status_int = CloudSyncStatus::DeletionPending.to_db_int();
        let rows = sqlx::query_as::<_, FileInfo>(
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
             FROM file_info
             WHERE cloud_sync_status = ? AND archive_file_name IS NOT NULL
             LIMIT ? OFFSET ?",
        )
        .bind(status_int)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;
        rows.into_iter()
            .map(CloudSyncableFileInfo::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Error::DecodeError(
                "Unexpected null archive_file_name in cloud deletion query".into(),
            ))
    }

    /// Counts DeletionPending files that have an archive_file_name.
    pub async fn count_cloud_files_pending_deletion(&self) -> Result<i64, Error> {
        let status_int = CloudSyncStatus::DeletionPending.to_db_int();
        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM file_info WHERE cloud_sync_status = ? AND archive_file_name IS NOT NULL",
            status_int
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(row.count)
    }

    /// Returns DeletionPending tombstones that have no archive_file_name (i.e. were never uploaded),
    /// paginated. These only require a DB delete — no cloud operation needed.
    pub async fn get_tombstones_pending_deletion(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<FileInfo>, Error> {
        let status_int = CloudSyncStatus::DeletionPending.to_db_int();
        Ok(sqlx::query_as::<_, FileInfo>(
            "SELECT id, sha1_checksum, file_size, archive_file_name, file_type, cloud_sync_status
             FROM file_info
             WHERE cloud_sync_status = ? AND archive_file_name IS NULL
             LIMIT ? OFFSET ?",
        )
        .bind(status_int)
        .bind(limit)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?)
    }

    /// Counts DeletionPending tombstones that have no archive_file_name.
    pub async fn count_tombstones_pending_deletion(&self) -> Result<i64, Error> {
        let status_int = CloudSyncStatus::DeletionPending.to_db_int();
        let row = sqlx::query!(
            "SELECT COUNT(*) as count FROM file_info WHERE cloud_sync_status = ? AND archive_file_name IS NULL",
            status_int
        )
        .fetch_one(&*self.pool)
        .await?;
        Ok(row.count)
    }

    /// Sets the `cloud_sync_status` for a `file_info` record.
    /// Returns `Err` if no record with the given `id` exists.
    pub async fn update_cloud_sync_status(
        &self,
        id: i64,
        status: CloudSyncStatus,
    ) -> Result<(), Error> {
        let status_int = status.to_db_int();
        let result = sqlx::query!(
            "UPDATE file_info SET cloud_sync_status = ? WHERE id = ?",
            status_int,
            id
        )
        .execute(&*self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(Error::DbError(format!(
                "file_info id {} not found",
                id
            )));
        }
        Ok(())
    }

    /// Returns the IDs of all `file_info` records with `cloud_sync_status = Synced`.
    /// Used by the file-type migration to determine which files exist in cloud storage.
    pub async fn get_synced_file_info_ids(&self) -> Result<std::collections::HashSet<i64>, Error> {
        let synced_int = CloudSyncStatus::Synced.to_db_int();
        let rows = sqlx::query!(
            "SELECT id FROM file_info WHERE cloud_sync_status = ?",
            synced_int
        )
        .fetch_all(&*self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.id).collect())
    }

    pub async fn delete_file_info(&self, id: i64) -> Result<(), Error> {
        let query = sqlx::query("DELETE FROM file_info WHERE id = ?").bind(id);
        query.execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn update_file_type(&self, id: i64, new_file_type: FileType) -> Result<(), Error> {
        let query = sqlx::query("UPDATE file_info SET file_type = ? WHERE id = ?")
            .bind(new_file_type.to_db_int())
            .bind(id);
        query.execute(&*self.pool).await?;
        Ok(())
    }

    /// Sets `archive_file_name` on an existing `file_info` record.
    /// Pass `Some(name)` to mark the file as available; `None` to clear it.
    pub async fn set_archive_file_name(
        &self,
        id: i64,
        archive_file_name: Option<&str>,
    ) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE file_info SET archive_file_name = ? WHERE id = ?",
            archive_file_name,
            id
        )
        .execute(&*self.pool)
        .await?;
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

    async fn insert_file_info(pool: &Pool<Sqlite>, archive_file_name: Option<&str>) -> i64 {
        let file_type = FileType::Rom.to_db_int();
        let checksum = vec![0u8; 20];
        let result = query!(
            "INSERT INTO file_info (sha1_checksum, file_size, archive_file_name, file_type)
             VALUES (?, ?, ?, ?)",
            checksum,
            1234,
            archive_file_name,
            file_type
        )
        .execute(pool)
        .await
        .unwrap();
        result.last_insert_rowid()
    }

    #[async_std::test]
    async fn test_get_files_pending_upload_returns_not_synced() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id = insert_file_info(&pool, Some("game.zst")).await;
        let results = repo
            .get_files_pending_upload(100, 0)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }

    #[async_std::test]
    async fn test_get_files_pending_upload_excludes_synced() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id = insert_file_info(&pool, Some("game.zst")).await;
        repo.update_cloud_sync_status(id, CloudSyncStatus::Synced)
            .await
            .unwrap();

        let pending = repo
            .get_files_pending_upload(100, 0)
            .await
            .unwrap();
        assert!(pending.is_empty());

        // Verify the record is indeed Synced
        let file_info = repo.get_file_info(id).await.unwrap();
        assert_eq!(file_info.cloud_sync_status, CloudSyncStatus::Synced);
    }

    #[async_std::test]
    async fn test_get_files_pending_upload_excludes_unavailable_files() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        insert_file_info(&pool, None).await;

        let results = repo
            .get_files_pending_upload(100, 0)
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[async_std::test]
    async fn test_get_files_pending_upload_respects_limit_and_offset() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        insert_file_info(&pool, Some("a.zst")).await;
        insert_file_info(&pool, Some("b.zst")).await;
        insert_file_info(&pool, Some("c.zst")).await;

        let page1 = repo
            .get_files_pending_upload(2, 0)
            .await
            .unwrap();
        let page2 = repo
            .get_files_pending_upload(2, 2)
            .await
            .unwrap();

        assert_eq!(page1.len(), 2);
        assert_eq!(page2.len(), 1);
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[async_std::test]
    async fn test_update_cloud_sync_status() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id = insert_file_info(&pool, Some("game.zst")).await;

        repo.update_cloud_sync_status(id, CloudSyncStatus::Synced)
            .await
            .unwrap();

        let file_info = repo.get_file_info(id).await.unwrap();
        assert_eq!(file_info.cloud_sync_status, CloudSyncStatus::Synced);

        repo.update_cloud_sync_status(id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        let file_info = repo.get_file_info(id).await.unwrap();
        assert_eq!(file_info.cloud_sync_status, CloudSyncStatus::DeletionPending);
    }

    #[async_std::test]
    async fn test_get_synced_file_info_ids() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id1 = insert_file_info(&pool, Some("a.zst")).await;
        let id2 = insert_file_info(&pool, Some("b.zst")).await;
        let _id3 = insert_file_info(&pool, Some("c.zst")).await;

        repo.update_cloud_sync_status(id1, CloudSyncStatus::Synced).await.unwrap();
        repo.update_cloud_sync_status(id2, CloudSyncStatus::DeletionPending).await.unwrap();

        let synced_ids = repo.get_synced_file_info_ids().await.unwrap();

        assert_eq!(synced_ids.len(), 1);
        assert!(synced_ids.contains(&id1));
        assert!(!synced_ids.contains(&id2));
    }

    #[async_std::test]
    async fn test_get_files_pending_deletion_returns_deletion_pending() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id = insert_file_info(&pool, Some("game.zst")).await;
        repo.update_cloud_sync_status(id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        let results = repo.get_files_pending_deletion(100, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
    }

    #[async_std::test]
    async fn test_get_files_pending_deletion_excludes_not_synced_and_synced() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id_not_synced = insert_file_info(&pool, Some("a.zst")).await;
        let id_synced = insert_file_info(&pool, Some("b.zst")).await;
        repo.update_cloud_sync_status(id_synced, CloudSyncStatus::Synced)
            .await
            .unwrap();
        let _ = id_not_synced;

        let results = repo.get_files_pending_deletion(100, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[async_std::test]
    async fn test_get_files_pending_deletion_includes_tombstones_without_archive_name() {
        // DeletionPending tombstones must be returned even without an archive_file_name,
        // because the deletion must be processed regardless.
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id = insert_file_info(&pool, None).await;
        repo.update_cloud_sync_status(id, CloudSyncStatus::DeletionPending)
            .await
            .unwrap();

        let results = repo.get_files_pending_deletion(100, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
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

    #[async_std::test]
    async fn test_set_archive_file_name_sets_value() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id = insert_file_info(&pool, None).await;
        repo.set_archive_file_name(id, Some("new.zst")).await.unwrap();

        let file_info = repo.get_file_info(id).await.unwrap();
        assert_eq!(file_info.archive_file_name.as_deref(), Some("new.zst"));
        assert!(file_info.is_available());
    }

    #[async_std::test]
    async fn test_set_archive_file_name_clears_to_none() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        let id = insert_file_info(&pool, Some("original.zst")).await;
        repo.set_archive_file_name(id, None).await.unwrap();

        let file_info = repo.get_file_info(id).await.unwrap();
        assert_eq!(file_info.archive_file_name, None);
        assert!(!file_info.is_available());
    }

    #[async_std::test]
    async fn test_count_files_pending_upload_excludes_unavailable_files() {
        let pool = setup_test_db().await;
        let repo = FileInfoRepository::new(Arc::new(pool.clone()));

        insert_file_info(&pool, None).await;

        let count = repo.count_files_pending_upload().await.unwrap();
        assert_eq!(count, 0);
    }
}
