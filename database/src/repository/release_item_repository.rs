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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository_manager::RepositoryManager;
    use crate::setup_test_db;
    use core_types::FileType;
    use core_types::item_type::ItemType;

    async fn get_repository() -> RepositoryManager {
        let pool = setup_test_db().await;
        RepositoryManager::new(Arc::new(pool))
    }

    #[async_std::test]
    async fn test_create_and_get_item() {
        let repository = get_repository().await;
        let release_id = repository
            .get_release_repository()
            .add_release("Test Release")
            .await
            .unwrap();

        let item_type = ItemType::Book;
        let notes = "Test notes".to_string();

        let release_item_repo = repository.get_release_item_repository();

        let item_id = release_item_repo
            .create_item(release_id, item_type, Some(notes.clone()))
            .await
            .unwrap();

        let item = release_item_repo.get_item(item_id).await.unwrap();
        assert_eq!(item.id, item_id);
        assert_eq!(item.release_id, release_id);
        assert_eq!(item.item_type, item_type);
        assert_eq!(item.notes, notes);

        let items = release_item_repo
            .get_items_for_release(release_id)
            .await
            .unwrap();
        assert_eq!(items.len(), 1);

        release_item_repo
            .update_item(item_id, Some("Updated notes".to_string()))
            .await
            .unwrap();

        let updated_item = release_item_repo.get_item(item_id).await.unwrap();
        assert_eq!(updated_item.notes, "Updated notes".to_string());

        let system_id = repository
            .get_system_repository()
            .add_system("Test System")
            .await
            .unwrap();

        let file_set_id = repository
            .get_file_set_repository()
            .add_file_set(
                "test_file_set",
                "test_file_set",
                &FileType::Rom,
                "Source",
                &[],
                &[system_id],
            )
            .await
            .unwrap();

        release_item_repo
            .link_file_set_to_item(item_id, file_set_id)
            .await
            .unwrap();
        let file_sets = release_item_repo
            .get_file_sets_for_item(item_id)
            .await
            .unwrap();
        assert_eq!(file_sets.len(), 1);
        assert_eq!(file_sets[0].id, file_set_id);

        let items_for_file_set = release_item_repo
            .get_items_for_file_set(file_set_id)
            .await
            .unwrap();
        assert_eq!(items_for_file_set.len(), 1);
        assert_eq!(items_for_file_set[0].id, item_id);

        release_item_repo
            .unlink_file_set_from_item(item_id, file_set_id)
            .await
            .unwrap();

        let file_sets_after_unlink = release_item_repo
            .get_file_sets_for_item(item_id)
            .await
            .unwrap();
        assert_eq!(file_sets_after_unlink.len(), 0);

        release_item_repo.delete_item(item_id).await.unwrap();
        let items_after_delete = release_item_repo
            .get_items_for_release(release_id)
            .await
            .unwrap();
        assert_eq!(items_after_delete.len(), 0);
    }
}
