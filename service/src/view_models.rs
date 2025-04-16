use std::{collections::HashMap, path::PathBuf};

use database::models::SettingName;
use file_system::get_files_root_dir;

pub struct EmulatorViewModel {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub systems: Vec<EmulatorSystemViewModel>,
}

pub struct EmulatorSystemViewModel {
    pub system_id: i64,
    pub system_name: String,
    pub arguments: String,
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub collection_root_dir: PathBuf,
}

impl From<HashMap<String, String>> for Settings {
    fn from(map: HashMap<String, String>) -> Self {
        let collection_root_dir = map
            .get(SettingName::CollectionRootDir.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(get_files_root_dir);
        Self {
            collection_root_dir,
        }
    }
}

fn get_default_folder() {}
