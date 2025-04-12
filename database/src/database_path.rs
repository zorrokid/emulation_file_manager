use directories_next::ProjectDirs;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Returns the database URL in the format sqlite:///absolute/path/to/db.sqlite
pub fn get_database_url() -> String {
    if let Ok(env_url) = env::var("DATABASE_URL") {
        return env_url;
    }

    let db_path = get_default_db_path();
    format!("sqlite://{}", db_path.display())
}

/// Returns the default path to db.sqlite in platform-appropriate user data dir
fn get_default_db_path() -> PathBuf {
    let project_dirs = ProjectDirs::from("com", "zorrokid", "softwarecollectionmanager")
        .expect("could not determine project directory");
    let data_dir = project_dirs.data_local_dir();
    fs::create_dir_all(data_dir).expect("failed to create app data directory");
    data_dir.join("db.sqlite")
}
