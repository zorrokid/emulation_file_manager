use std::{env, sync::Arc};

use dotenvy::dotenv;
use sqlx::{Pool, Sqlite, SqlitePool};

pub async fn get_db_pool() -> Result<Arc<Pool<Sqlite>>, sqlx::Error> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    let pool = SqlitePool::connect(&db_url).await?;
    Ok(Arc::new(pool))
}

pub async fn get_memory_db_pool() -> Result<Pool<Sqlite>, sqlx::Error> {
    SqlitePool::connect("sqlite::memory:").await
}
