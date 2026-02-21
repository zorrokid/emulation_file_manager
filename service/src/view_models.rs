use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    path::PathBuf,
};

use core_types::{
    ArgumentType, DocumentType, FileSize, FileType, SettingName, Sha1Checksum, item_type::ItemType,
};
use database::models::{
    DocumentViewer, Emulator, FileSet, FileSetFileInfo, ReleaseExtended, ReleaseItem,
    SoftwareTitle, System,
};
use file_system::get_files_root_dir;

#[derive(Debug, Clone, PartialEq)]
pub struct EmulatorViewModel {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub arguments: Vec<ArgumentType>,
    pub system: SystemListModel,
}

impl Display for EmulatorViewModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
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

#[derive(Debug, Clone, Default)]
pub struct S3Settings {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub collection_root_dir: PathBuf,
    pub temp_output_dir: PathBuf,
    pub s3_settings: Option<S3Settings>,
    pub s3_sync_enabled: bool,
}

impl Settings {
    /// Get the path to a specific file type directory within the collection root
    pub fn get_file_type_path(&self, file_type: &FileType) -> PathBuf {
        self.collection_root_dir.join(file_type.dir_name())
    }

    /// Get the full path to a specific file within the collection
    /// Automatically appends the .zst extension to the archive_file_name
    pub fn get_file_path(&self, file_type: &FileType, archive_file_name: &str) -> PathBuf {
        self.get_file_type_path(file_type)
            .join(archive_file_name)
            .with_extension("zst")
    }

    pub fn get_thumbnails_path(&self) -> PathBuf {
        self.collection_root_dir.join("thumbnails")
    }
}

impl From<HashMap<String, String>> for Settings {
    fn from(map: HashMap<String, String>) -> Self {
        let collection_root_dir = map
            .get(SettingName::CollectionRootDir.as_str())
            .map(PathBuf::from)
            .unwrap_or_else(get_files_root_dir);
        let s3_endpoint = map.get(SettingName::S3EndPoint.as_str());
        let s3_region = map.get(SettingName::S3Region.as_str());
        let s3_bucket = map.get(SettingName::S3Bucket.as_str());
        let s3_settings = match (s3_endpoint, s3_region, s3_bucket) {
            (Some(endpoint), Some(region), Some(bucket)) => Some(S3Settings {
                endpoint: endpoint.clone(),
                region: region.clone(),
                bucket: bucket.clone(),
            }),
            _ => None,
        };
        let s3_sync_enabled = map
            .get(SettingName::S3FileSyncEnabled.as_str())
            .map(|v| v == "true")
            .unwrap_or(false);
        Self {
            collection_root_dir,
            temp_output_dir: std::env::temp_dir(),
            s3_settings,
            s3_sync_enabled,
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
    pub file_name: String,
    pub can_delete: bool,
}

impl From<&FileSet> for FileSetListModel {
    fn from(file_set: &FileSet) -> Self {
        FileSetListModel {
            id: file_set.id,
            file_name: file_set.file_name.clone(),
            file_type: file_set.file_type,
            file_set_name: file_set.name.clone(),
            can_delete: false,
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
    pub files: Vec<FileSetFileInfoViewModel>,
    pub file_name: String,
    pub source: String,
    pub can_delete: bool,
    pub item_types: Vec<ItemType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseItemViewModel {
    pub id: i64,
    pub item_type: ItemType,
    pub notes: String,
}

impl From<ReleaseItem> for ReleaseItemViewModel {
    fn from(release_item: ReleaseItem) -> Self {
        ReleaseItemViewModel {
            id: release_item.id,
            item_type: release_item.item_type,
            notes: release_item.notes.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseItemListModel {
    pub id: i64,
    pub item_type: ItemType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileSetFileInfoViewModel {
    pub file_set_id: i64,
    pub file_info_id: i64,
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,
    pub archive_file_name: String,
    pub file_type: FileType,
}

impl From<&FileSetFileInfo> for FileSetFileInfoViewModel {
    fn from(file_set_file_info: &FileSetFileInfo) -> Self {
        FileSetFileInfoViewModel {
            file_set_id: file_set_file_info.file_set_id,
            file_info_id: file_set_file_info.file_info_id,
            file_name: file_set_file_info.file_name.clone(),
            sha1_checksum: file_set_file_info
                .sha1_checksum
                .clone()
                .try_into()
                .expect("Invalid SHA1 checksum length"),
            file_size: file_set_file_info.file_size as u64,
            archive_file_name: file_set_file_info.archive_file_name.clone(),
            file_type: file_set_file_info.file_type,
        }
    }
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

#[derive(Debug, Clone)]
pub struct FileInfoViewModel {
    pub id: i64,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: u64,
    pub archive_file_name: String,
    pub belongs_to_file_sets: Vec<FileSetListModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseListModel {
    pub id: i64,
    pub name: String,
    pub system_names: Vec<String>,
    pub file_types: Vec<String>,
    pub software_title_names: Vec<String>,
    pub media_file_types: Vec<String>,
}

impl From<&ReleaseExtended> for ReleaseListModel {
    fn from(release: &ReleaseExtended) -> Self {
        ReleaseListModel {
            id: release.id,
            name: release.name.clone(),
            system_names: release.system_names.clone(),
            file_types: release.file_types.iter().map(|ft| ft.to_string()).collect(),
            software_title_names: release.software_title_names.clone(),
            media_file_types: release
                .file_types
                .iter()
                .filter(|ft| ft.is_media_type())
                .map(|ft| ft.to_string())
                .collect(),
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
    pub systems: Vec<domain::models::System>,
    pub software_titles: Vec<SoftwareTitle>,
    pub file_sets: Vec<FileSetViewModel>,
    pub items: Vec<ReleaseItemViewModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentViewerListModel {
    pub id: i64,
    pub name: String,
}

impl From<&DocumentViewer> for DocumentViewerListModel {
    fn from(document_viewer: &DocumentViewer) -> Self {
        DocumentViewerListModel {
            id: document_viewer.id,
            name: document_viewer.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentViewerViewModel {
    pub id: i64,
    pub name: String,
    pub executable: String,
    pub arguments: Vec<ArgumentType>,
    pub document_type: DocumentType,
    pub cleanup_temp_files: bool,
}

impl Display for DocumentViewerViewModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
