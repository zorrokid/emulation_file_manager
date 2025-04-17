use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use database::models::{SettingName, System};
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

#[derive(Debug, Clone, PartialEq)]
pub struct SystemListModel {
    pub id: i64,
    pub name: String,
    pub can_delete: bool,
}

impl From<&System> for SystemListModel {
    fn from(system: &System) -> Self {
        SystemListModel {
            id: system.id,
            name: system.name.clone(),
            can_delete: false,
        }
    }
}

impl Display for SystemListModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
