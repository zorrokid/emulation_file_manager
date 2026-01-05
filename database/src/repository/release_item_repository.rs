use std::sync::Arc;

use sqlx::{Pool, Sqlite};

#[derive(Debug)]
pub struct ReleaseItemRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl ReleaseItemRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}
