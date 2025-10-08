use std::sync::Arc;

use core_types::FileSyncStatus;
use sqlx::{prelude::FromRow, sqlite::SqliteRow, Pool, QueryBuilder, Row, Sqlite};

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

    pub async fn get_logs_and_file_info_by_sync_status(
        &self,
        limit: i64,
        status: FileSyncStatus,
    ) -> Result<Vec<FileSyncLogWithFileInfo>, sqlx::Error> {
        let logs = sqlx::query_as::<_, FileSyncLogWithFileInfo>(
            "SELECT log.id, log.file_info_id, log.sync_time, log.status, log.message, log.cloud_key, fi.sha1_checksum, fi.file_size, fi.archive_file_name, fi.file_type 
             FROM file_sync_log log
             INNER JOIN file_info fi ON log.file_info_id = fi.id
             WHERE log.status = ?
             LIMIT ?",
        )
        .bind(status.to_db_int())
        .bind(limit)
        .fetch_all(&*self.pool)
        .await?;
        Ok(logs)
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

    pub async fn update_log_entry(
        &self,
        log_id: i64,
        status: FileSyncStatus,
        message: &str,
    ) -> Result<(), sqlx::Error> {
        let status = status.to_db_int();
        sqlx::query!(
            "UPDATE file_sync_log 
             SET status = ?, message = ?, sync_time = datetime('now')
             WHERE id = ?",
            status,
            message,
            log_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }
}
