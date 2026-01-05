use std::sync::Arc;

use sqlx::{Pool, Sqlite};

use crate::models::{FileSet, ReleaseItem};

#[derive(Debug)]
pub struct ReleaseItemRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl ReleaseItemRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
    /*
    CREATE TABLE release_item (
        id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
        release_id INTEGER NOT NULL,
        item_type INTEGER NOT NULL,
        notes TEXT,
        FOREIGN KEY (release_id) REFERENCES release(id) ON DELETE CASCADE
    );

    CREATE TABLE file_set_item (
        file_set_id INTEGER NOT NULL,
        item_id INTEGER NOT NULL,
        PRIMARY KEY (file_set_id, item_id),
        FOREIGN KEY (file_set_id) REFERENCES file_set(id) ON DELETE CASCADE,
        FOREIGN KEY (item_id) REFERENCES release_item(id) ON DELETE CASCADE
    );
      */
    pub fn create_item(
        &self,
        release_id: i64,
        item_type: u8,
        notes: Option<String>,
    ) -> Result<i64, sqlx::Error> {
        unimplemented!();
    }

    pub fn get_item(&self, item_id: i64) -> Result<ReleaseItem, sqlx::Error> {
        unimplemented!();
    }

    pub fn get_items_for_release(&self, release_id: i64) -> Result<Vec<ReleaseItem>, sqlx::Error> {
        unimplemented!();
    }

    pub fn update_item(&self, item_id: i64, notes: Option<String>) -> Result<(), sqlx::Error> {
        unimplemented!();
    }

    pub fn delete_item(&self, item_id: i64) -> Result<(), sqlx::Error> {
        unimplemented!();
    }

    pub fn link_file_set_to_item(&self, item_id: i64, file_set_id: i64) -> Result<(), sqlx::Error> {
        unimplemented!();
    }

    pub fn unlink_file_set_from_item(
        &self,
        item_id: i64,
        file_set_id: i64,
    ) -> Result<(), sqlx::Error> {
        unimplemented!();
    }

    // TODO: should this be in FileSetRepository?
    pub fn get_file_sets_for_item(&self, item_id: i64) -> Result<Vec<FileSet>, sqlx::Error> {
        unimplemented!();
    }

    pub fn get_items_for_file_set(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<ReleaseItem>, sqlx::Error> {
        unimplemented!();
    }
}
