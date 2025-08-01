use std::{fs, path::PathBuf};

use directories_next::ProjectDirs;

/// Returns path to database file located in default data dir for application.
pub fn get_database_path() -> PathBuf {
    get_default_data_dir().join("db.sqlite")
}

/// Returns path to files directory located in default data fir for application.
pub fn get_files_root_dir() -> PathBuf {
    get_default_data_dir().join("files")
}

fn get_default_data_dir() -> std::path::PathBuf {
    let project_dirs = get_project_dirs();
    let data_dir = project_dirs.data_local_dir();
    fs::create_dir_all(data_dir).expect("Failed to create app data directory");
    std::path::PathBuf::from(data_dir)
}

fn get_project_dirs() -> ProjectDirs {
    ProjectDirs::from("org", "zorrokid", "efm").expect("could not determine project directory")
}
