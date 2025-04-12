mod database_error;
mod database_path;
mod models;
mod repository;

use std::sync::Arc;

use sqlx::{migrate, Pool, Sqlite, SqlitePool};

pub async fn get_db_pool() -> Result<Arc<Pool<Sqlite>>, sqlx::Error> {
    /*let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let db_path = base_dir.join("data/db.sqlite");
    let db_url = format!("sqlite://{}", db_path.display());*/
    let db_url = database_path::get_database_url();
    let pool = SqlitePool::connect(&db_url).await?;
    Ok(Arc::new(pool))
}

pub async fn get_memory_db_pool() -> Result<Pool<Sqlite>, sqlx::Error> {
    SqlitePool::connect("sqlite::memory:").await
}

pub async fn setup_test_db() -> SqlitePool {
    // Set the DATABASE_URL for testing

    // Create an in-memory database connection
    let pool = SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to the in-memory SQLite database");

    // Run migrations
    migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}
