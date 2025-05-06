use file_system::get_database_path;
use std::{env, path::PathBuf};

/// Returns the database URL in the format sqlite:///absolute/path/to/db.sqlite
pub fn get_database_url() -> String {
    if let Ok(env_url) = env::var("DATABASE_URL") {
        return env_url;
    }

    let db_path = get_database_path();

    format!("sqlite://{}", db_path.display())
}

pub fn get_database_file_path() -> PathBuf {
    let db_path = get_database_path();
    println!("Database path: {}", db_path.display());
    db_path
}
