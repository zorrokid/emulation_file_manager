use std::sync::Arc;

use sqlx::{Pool, Sqlite};

pub struct SoftwareTitleRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl SoftwareTitleRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}
