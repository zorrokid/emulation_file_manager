use std::sync::Arc;

use core_types::item_type::ItemType;
use sqlx::{Pool, Row, Sqlite, prelude::FromRow, sqlite::SqliteRow};

use crate::models::{FileSet, ReleaseItem};

impl FromRow<'_, SqliteRow> for ReleaseItem {
    fn from_row(row: &SqliteRow) -> Result<Self, sqlx::Error> {
        let item_type_int: u8 = row.try_get("item_type")?;
        let item_type: ItemType =
            ItemType::from_db_int(item_type_int).expect("Invalid item type in DB");
        Ok(Self {
            id: row.try_get("id")?,
            release_id: row.try_get("release_id")?,
            item_type,
            notes: row.try_get("notes")?,
        })
    }
}

#[derive(Debug)]
pub struct ReleaseItemRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl ReleaseItemRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn create_item(
        &self,
        release_id: i64,
        item_type: ItemType,
        notes: Option<String>,
    ) -> Result<i64, sqlx::Error> {
        let item_type = item_type.to_db_int();
        let result = sqlx::query!(
            "INSERT INTO release_item (
                release_id,
                item_type,
                notes
            ) VALUES (?, ?, ?)",
            release_id,
            item_type,
            notes
        )
        .execute(&*self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_item(&self, item_id: i64) -> Result<ReleaseItem, sqlx::Error> {
        let item = sqlx::query_as::<_, ReleaseItem>(
            "SELECT 
                id,
                release_id,
                item_type,
                notes
            FROM release_item
            WHERE id = ?",
        )
        .bind(item_id)
        .fetch_one(&*self.pool)
        .await?;
        Ok(item)
    }

    pub async fn get_items_for_release(
        &self,
        release_id: i64,
    ) -> Result<Vec<ReleaseItem>, sqlx::Error> {
        let items = sqlx::query_as::<_, ReleaseItem>(
            "SELECT 
                id,
                release_id,
                item_type,
                notes
            FROM release_item
            WHERE release_id = ?",
        )
        .bind(release_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(items)
    }

    pub async fn update_item(
        &self,
        item_id: i64,
        notes: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE release_item
            SET notes = ?
            WHERE id = ?",
            notes,
            item_id
        )
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_item(&self, item_id: i64) -> Result<(), sqlx::Error> {
        let query = sqlx::query("DELETE FROM release_item WHERE id = ?").bind(item_id);
        query.execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn link_file_set_to_item(
        &self,
        item_id: i64,
        file_set_id: i64,
    ) -> Result<(), sqlx::Error> {
        let query = sqlx::query(
            "INSERT INTO file_set_item (
                file_set_id,
                item_id
            ) VALUES (?, ?)",
        )
        .bind(file_set_id)
        .bind(item_id);
        query.execute(&*self.pool).await?;
        Ok(())
    }

    pub async fn unlink_file_set_from_item(
        &self,
        item_id: i64,
        file_set_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM file_set_item
            WHERE file_set_id = ? AND item_id = ?",
        )
        .bind(file_set_id)
        .bind(item_id)
        .execute(&*self.pool)
        .await?;
        Ok(())
    }

    // TODO: should this be in FileSetRepository?
    pub async fn get_file_sets_for_item(&self, item_id: i64) -> Result<Vec<FileSet>, sqlx::Error> {
        let file_sets = sqlx::query_as::<_, FileSet>(
            "SELECT 
                fs.id,
                fs.file_name,
                fs.file_type,
                fs.name,
                fs.source
            FROM file_set fs
            JOIN file_set_item fsi ON fs.id = fsi.file_set_id
            WHERE fsi.item_id = ?",
        )
        .bind(item_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(file_sets)
    }

    pub async fn get_items_for_file_set(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<ReleaseItem>, sqlx::Error> {
        let items = sqlx::query_as::<_, ReleaseItem>(
            "SELECT 
                ri.id,
                ri.release_id,
                ri.item_type,
                ri.notes
            FROM release_item ri
            JOIN file_set_item fsi ON ri.id = fsi.item_id
            WHERE fsi.file_set_id = ?",
        )
        .bind(file_set_id)
        .fetch_all(&*self.pool)
        .await?;
        Ok(items)
    }
}
