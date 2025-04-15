use std::sync::Arc;

use sqlx::{Pool, Sqlite};

pub struct ReleaseRepository {
    pool: Arc<Pool<Sqlite>>,
}

impl ReleaseRepository {
    pub fn new(pool: Arc<Pool<Sqlite>>) -> Self {
        Self { pool }
    }
}
