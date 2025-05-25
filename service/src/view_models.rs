use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use database::models::{
    Emulator, FileInfo, FileSet, FileSetFileInfo, FileType, ReleaseExtended, SettingName,
    SoftwareTitle, System,
};
use file_system::get_files_root_dir;

#[derive(Debug, Clone)]
pub struct EmulatorViewModel {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub systems: Vec<EmulatorSystemViewModel>,
}

#[derive(Debug, Clone)]
pub struct EmulatorSystemViewModel {
    pub id: i64,
    pub system_id: i64,
    pub system_name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmulatorListModel {
    pub id: i64,
    pub name: String,
}

impl From<&Emulator> for EmulatorListModel {
    fn from(emulator: &Emulator) -> Self {
        EmulatorListModel {
            id: emulator.id,
            name: emulator.name.clone(),
        }
    }
}

impl Display for EmulatorListModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct EmulatorSystemListModel {
    pub id: i64,
    pub system_name: String,
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

#[derive(Debug, Clone, PartialEq)]
pub struct SoftwareTitleListModel {
    pub id: i64,
    pub name: String,
    pub can_delete: bool,
}

impl From<&SoftwareTitle> for SoftwareTitleListModel {
    fn from(software_title: &SoftwareTitle) -> Self {
        SoftwareTitleListModel {
            id: software_title.id,
            name: software_title.name.clone(),
            can_delete: false,
        }
    }
}

impl Display for SoftwareTitleListModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileSetListModel {
    pub id: i64,
    pub file_set_name: String,
    pub file_type: FileType,
}

impl From<&FileSet> for FileSetListModel {
    fn from(file_set: &FileSet) -> Self {
        FileSetListModel {
            id: file_set.id,
            file_set_name: file_set.file_name.clone(),
            file_type: file_set.file_type,
        }
    }
}

impl Display for FileSetListModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.file_set_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileSetViewModel {
    pub id: i64,
    pub file_set_name: String,
    pub file_type: FileType,
    pub files: Vec<FileSetFileInfo>,
}

impl Display for FileSetViewModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.file_set_name, self.file_type)
    }
}

pub struct FileSetFileViewModel {
    pub id: i64,
    pub file_name: String,
    pub file_type: FileType,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseListModel {
    pub id: i64,
    pub name: String,
    pub system_names: Vec<String>,
    pub file_types: Vec<String>,
}

impl From<&ReleaseExtended> for ReleaseListModel {
    fn from(release: &ReleaseExtended) -> Self {
        ReleaseListModel {
            id: release.id,
            name: release.name.clone(),
            system_names: release.system_names.clone(),
            file_types: release.file_types.iter().map(|ft| ft.to_string()).collect(),
        }
    }
}

impl Display for ReleaseListModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.name, self.system_names.join(", "))
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseViewModel {
    pub id: i64,
    pub name: String,
    pub systems: Vec<System>,
    pub software_titles: Vec<SoftwareTitle>,
    pub file_sets: Vec<FileSetViewModel>,
}
