pub mod database_error;
mod database_path;
pub mod models;
mod repository;
pub mod repository_manager;

use std::sync::Arc;

use database_path::get_database_file_path;
use sqlx::{migrate, sqlite::SqliteConnectOptions, Pool, Sqlite, SqlitePool};

pub async fn get_db_pool() -> Result<Arc<Pool<Sqlite>>, sqlx::Error> {
    let db_file_path = get_database_file_path();
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(db_file_path)
            .create_if_missing(true),
    )
    .await?;
    sqlx::migrate!().run(&pool).await?;
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
