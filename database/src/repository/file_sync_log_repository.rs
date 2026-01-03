use std::sync::Arc;

use core_types::FileSyncStatus;
use sqlx::{Pool, Row, Sqlite, prelude::FromRow, sqlite::SqliteRow};

use crate::models::{FileSyncLog, FileSyncLogWithFileInfo};

impl FromRow<'_, SqliteRow> for FileSyncLog {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let file_sync_status: u8 = row.try_get("status")?;
        let status: FileSyncStatus =
            FileSyncStatus::from_db_int(file_sync_status).expect("Invalid file sync status in DB");
        Ok(Self {
            id: row.try_get("id")?,
            file_info_id: row.try_get("file_info_id")?,
            sync_time: row.try_get("sync_time")?,
            status,
            message: row.try_get("message")?,
            cloud_key: row.try_get("cloud_key")?,
        })
    }
}

impl FromRow<'_, SqliteRow> for FileSyncLogWithFileInfo {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let file_sync_status_int: u8 = row.try_get("status")?;
        let file_sync_status: FileSyncStatus = FileSyncStatus::from_db_int(file_sync_status_int)
            .expect("Invalid file sync status in DB");
        let file_type_int = row.try_get("file_type")?;
        let file_type =
            core_types::FileType::from_db_int(file_type_int).expect("Invalid file type in DB");
        Ok(Self {
            id: row.try_get("id")?,
            file_info_id: row.try_get("file_info_id")?,
            sync_time: row.try_get("sync_time")?,
            status: file_sync_status,
            message: row.try_get("message")?,
            cloud_key: row.try_get("cloud_key")?,
            sha1_checksum: row.try_get("sha1_checksum")?,
            file_size: row.try_get("file_size")?,
            archive_file_name: row.try_get("archive_file_name")?,
            file_type,
        })
    }
}

#[derive(Debug)]
pub struct FileSyncLogRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl FileSyncLogRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn get_logs_by_file_info(
        &self,
        file_info_id: i64,
    ) -> Result<Vec<FileSyncLog>, sqlx::Error> {
        let logs = sqlx::query_as::<_, FileSyncLog>(
            "SELECT id, file_info_id, sync_time, status, message, cloud_key 
             FROM file_sync_log
             WHERE file_info_id = ?
             ORDER BY id DESC",
        )
        .bind(file_info_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(logs)
    }

    /// Fetches the most recent log entries for each file_info_id where the status is either
    /// UploadPending or UploadFailed, with pagination support.
    /// This is useful for identifying files that need to be retried for upload.
    /// Note: if user has a later log entry with different status it will be excluded from result
    /// set.
    pub async fn get_logs_and_file_info_by_sync_status(
        &self,
        statuses: &[FileSyncStatus],
        limit: u32,
        offset: u32,
    ) -> Result<Vec<FileSyncLogWithFileInfo>, sqlx::Error> {
        let mut query_builder = sqlx::QueryBuilder::<Sqlite>::new(
            "SELECT log.id, log.file_info_id, log.sync_time, log.status, log.message, log.cloud_key, fi.sha1_checksum, fi.file_size, fi.archive_file_name, fi.file_type 
             FROM file_sync_log log
             INNER JOIN file_info fi ON log.file_info_id = fi.id
             INNER JOIN (
                SELECT file_info_id, MAX(id) AS max_id
                FROM file_sync_log
                GROUP BY file_info_id
             ) latest ON log.file_info_id = latest.file_info_id AND log.id = latest.max_id
             WHERE log.status IN (");
        let mut separated = query_builder.separated(", ");
        for status in statuses {
            separated.push_bind(status.to_db_int());
        }
        separated.push_unseparated(
            ") ORDER BY log.sync_time DESC
             LIMIT ? OFFSET ?",
        );

        let query = query_builder
            .build_query_as::<FileSyncLogWithFileInfo>()
            .bind(limit)
            .bind(offset);
        let entries = query.fetch_all(&*self.pool).await?;
        Ok(entries)
    }

    pub async fn count_logs_by_latest_statuses(
        &self,
        statuses: &[FileSyncStatus],
    ) -> Result<i64, sqlx::Error> {
        let mut query_builder = sqlx::QueryBuilder::<Sqlite>::new(
            "SELECT COUNT(*) as count FROM (
                -- Get the latest log entry for each file_info_id
                SELECT file_info_id, MAX(id) AS max_id
                FROM file_sync_log
                GROUP BY file_info_id
             ) latest
             -- Join with the file_sync_log table to get the status of the latest entries
             INNER JOIN file_sync_log log ON latest.file_info_id = log.file_info_id AND latest.max_id = log.id
             -- And finally filter by the desired status
             WHERE log.status IN (");
        let mut separated = query_builder.separated(", ");
        for status in statuses {
            separated.push_bind(status.to_db_int());
        }
        separated.push_unseparated(")");

        let query = query_builder.build();
        let row = query.fetch_one(&*self.pool).await?;
        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    pub async fn add_log_entry(
        &self,
        file_info_id: i64,
        status: FileSyncStatus,
        message: &str,
        cloud_key: &str,
    ) -> Result<i64, sqlx::Error> {
        let status = status.to_db_int();
        let result = sqlx::query!(
            "INSERT INTO file_sync_log (file_info_id, sync_time, status, message, cloud_key)
             VALUES (?, datetime('now'), ?, ?, ?)",
            file_info_id,
            status,
            message,
            cloud_key
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn mark_files_for_cloud_sync(
        &self,
        file_info_ids: &[(i64, String)],
    ) -> Result<(), sqlx::Error> {
        if file_info_ids.is_empty() {
            return Ok(());
        }
        let mut query_builder = sqlx::QueryBuilder::<Sqlite>::new(
            "INSERT INTO file_sync_log (file_info_id, status, cloud_key) ",
        );
        query_builder.push_values(file_info_ids.iter(), |mut b, (file_info_id, cloud_key)| {
            b.push_bind(file_info_id)
                .push_bind(FileSyncStatus::UploadPending.to_db_int())
                .push_bind(cloud_key);
        });
        let query = query_builder.build();
        query.execute(&*self.pool).await?;
        Ok(())
    }

    /// Clean up sync log entries for file_info records that no longer exist
    pub async fn cleanup_orphaned_logs(&self) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM file_sync_log 
             WHERE file_info_id NOT IN (SELECT id FROM file_info)"
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use core_types::Sha1Checksum;

    use super::*;
    use crate::{repository::file_info_repository::FileInfoRepository, setup_test_db};

    #[async_std::test]
    async fn test_get_logs_and_file_info_by_sync_status() {
        let pool = Arc::new(setup_test_db().await);
        let repository = FileSyncLogRepository::new(Arc::clone(&pool));
        let file_info_id = insert_file_info(&pool).await;
        repository
            .add_log_entry(file_info_id, FileSyncStatus::UploadPending, "", "")
            .await
            .unwrap();
        repository
            .add_log_entry(file_info_id, FileSyncStatus::UploadFailed, "", "")
            .await
            .unwrap();
        let res = repository
            .get_logs_and_file_info_by_sync_status(&[FileSyncStatus::UploadFailed], 1, 0)
            .await
            .unwrap();
        assert_eq!(res.len(), 1);

        // add another file_info and log entries for it
        let file_info_id_2 = insert_file_info(&pool).await;
        repository
            .add_log_entry(file_info_id_2, FileSyncStatus::UploadPending, "", "")
            .await
            .unwrap();
        repository
            .add_log_entry(file_info_id_2, FileSyncStatus::UploadFailed, "", "")
            .await
            .unwrap();

        // test pagination
        let res = repository
            .get_logs_and_file_info_by_sync_status(&[FileSyncStatus::UploadFailed], 1, 0)
            .await
            .unwrap();
        assert_eq!(res.len(), 1);
        let first = res.first().unwrap();
        assert_eq!(first.file_info_id, file_info_id);
        let res = repository
            .get_logs_and_file_info_by_sync_status(&[FileSyncStatus::UploadFailed], 1, 1)
            .await
            .unwrap();
        assert_eq!(res.len(), 1);
        let first = res.first().unwrap();
        assert_eq!(first.file_info_id, file_info_id_2);

        // add one more log entry
        repository
            .add_log_entry(file_info_id, FileSyncStatus::DeletionPending, "", "")
            .await
            .unwrap();
        repository
            .add_log_entry(file_info_id_2, FileSyncStatus::DeletionPending, "", "")
            .await
            .unwrap();

        // now UploadFailed status shouldn't return anything since there is more recent log entry
        // for both files with different status
        //
        let res = repository
            .get_logs_and_file_info_by_sync_status(&[FileSyncStatus::UploadFailed], 10, 0)
            .await
            .unwrap();
        assert_eq!(res.len(), 0);
    }

    #[async_std::test]
    async fn test_count_logs_by_latest_statuses() {
        let pool = Arc::new(setup_test_db().await);
        let repository = FileSyncLogRepository::new(Arc::clone(&pool));
        let file_info_id = insert_file_info(&pool).await;
        repository
            .add_log_entry(file_info_id, FileSyncStatus::UploadPending, "", "")
            .await
            .unwrap();
        repository
            .add_log_entry(file_info_id, FileSyncStatus::UploadFailed, "", "")
            .await
            .unwrap();

        let count = repository
            .count_logs_by_latest_statuses(&[FileSyncStatus::UploadFailed])
            .await
            .unwrap();
        assert_eq!(count, 1);

        let count = repository
            .count_logs_by_latest_statuses(&[FileSyncStatus::UploadPending])
            .await
            .unwrap();
        assert_eq!(count, 0);

        // add another file_info and log entries for it
        let file_info_id_2 = insert_file_info(&pool).await;
        repository
            .add_log_entry(file_info_id_2, FileSyncStatus::UploadPending, "", "")
            .await
            .unwrap();
        repository
            .add_log_entry(file_info_id_2, FileSyncStatus::UploadFailed, "", "")
            .await
            .unwrap();
        let count = repository
            .count_logs_by_latest_statuses(&[FileSyncStatus::UploadFailed])
            .await
            .unwrap();
        assert_eq!(count, 2);

        // add one more log entry with different status for first file_info
        repository
            .add_log_entry(file_info_id, FileSyncStatus::DeletionPending, "", "")
            .await
            .unwrap();

        let count = repository
            .count_logs_by_latest_statuses(&[FileSyncStatus::UploadFailed])
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[async_std::test]
    async fn test_cleanup_orphaned_logs() {
        let pool = Arc::new(setup_test_db().await);
        let repository = FileSyncLogRepository::new(Arc::clone(&pool));
        let file_info_id = insert_file_info(&pool).await;
        repository
            .add_log_entry(file_info_id, FileSyncStatus::UploadCompleted, "", "")
            .await
            .unwrap();

        // delete the file_info record to create an orphaned log
        sqlx::query!("DELETE FROM file_info WHERE id = ?", file_info_id)
            .execute(&*pool)
            .await
            .unwrap();

        let deleted_count = repository.cleanup_orphaned_logs().await.unwrap();
        assert_eq!(deleted_count, 1);
    }

    #[async_std::test]
    async fn test_mark_files_for_cloud_sync() {
        let pool = Arc::new(setup_test_db().await);
        let repository = FileSyncLogRepository::new(Arc::clone(&pool));
        let file_info_id_1 = insert_file_info(&pool).await;
        let file_info_id_2 = insert_file_info(&pool).await;

        let file_info_repository = FileInfoRepository::new(Arc::clone(&pool));
        let file_info_1 = file_info_repository
            .get_file_info(file_info_id_1)
            .await
            .unwrap();

        let file_info_2 = file_info_repository
            .get_file_info(file_info_id_2)
            .await
            .unwrap();

        repository
            .mark_files_for_cloud_sync(&[
                (file_info_id_1, file_info_1.generate_cloud_key()),
                (file_info_id_2, file_info_2.generate_cloud_key()),
            ])
            .await
            .unwrap();

        let logs_1 = repository
            .get_logs_by_file_info(file_info_id_1)
            .await
            .unwrap();
        assert_eq!(logs_1.len(), 1);
        assert_eq!(logs_1[0].status, FileSyncStatus::UploadPending);

        let logs_2 = repository
            .get_logs_by_file_info(file_info_id_2)
            .await
            .unwrap();
        assert_eq!(logs_2.len(), 1);
        assert_eq!(logs_2[0].status, FileSyncStatus::UploadPending);
    }

    async fn insert_file_info(pool: &Pool<Sqlite>) -> i64 {
        let sha1_checksum: Sha1Checksum = [0u8; 20];
        let sha1_checksum_bytes = sha1_checksum.to_vec();
        let result = sqlx::query!(
            "INSERT INTO file_info (
                sha1_checksum,
                file_size,
                archive_file_name,
                file_type
            ) VALUES (?, ?, ?, ?)",
            sha1_checksum_bytes,
            1,
            "test_file_1",
            1
        )
        .execute(pool)
        .await
        .unwrap();

        result.last_insert_rowid()
    }
}
