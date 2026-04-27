use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use libretro_runner::supported_cores::{InputProfile, get_supported_core};

use crate::{
    error::Error, file_system_ops::FileSystemOps, libretro_runner::service::LibretroPreflightError,
    view_models::Settings,
};

pub use libretro_runner::model::LibretroSystemInfo;

#[derive(Debug)]
pub struct CoreMappingModel {
    pub id: i64,
    pub core_name: String,
}

#[derive(Debug)]
pub struct SystemCoreMappingModel {
    pub id: i64,
    pub system_id: i64,
    pub system_name: String,
}

pub struct LibretroCoreService {
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug, Clone)]
pub struct LibretroFirmwareInfo {
    pub desc: String,
    pub path: String,
    pub opt: bool,
    pub available: bool,
}

#[derive(Debug, Clone)]
pub struct LibretroCoreInfo {
    pub core_name: String,
    pub is_available: bool,
    pub firmware_info: Vec<LibretroFirmwareInfo>,
    pub input_profile: InputProfile,
    pub supported_extensions: Vec<String>,
}

impl LibretroCoreInfo {
    fn has_required_firmware(&self) -> bool {
        self.firmware_info
            .iter()
            .filter(|f| !f.opt)
            .all(|f| f.available)
    }

    pub fn can_launch(&self) -> bool {
        self.is_available && self.has_required_firmware()
    }
}

impl std::fmt::Debug for LibretroCoreService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibretroCoreService")
            .field("settings", &"Settings { ... }")
            .field("fs_ops", &"FileSystemOps { ... }")
            .finish()
    }
}

impl LibretroCoreService {
    pub fn new(
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
        repository_manager: Arc<RepositoryManager>,
    ) -> Self {
        Self {
            settings,
            fs_ops,
            repository_manager,
        }
    }

    // TODO: should be async
    pub fn list_cores(&self) -> Result<Vec<String>, Error> {
        if let Some(libretro_core_dir) = &self.settings.libretro_core_dir {
            let result = self.fs_ops.read_dir(libretro_core_dir);

            match result {
                Ok(entries) => {
                    let cores = entries
                        .into_iter()
                        .filter_map(|entry| {
                            if let Ok(entry) = entry
                                // TODO: should we also check that info file is present?
                                && self.fs_ops.is_file(&entry.path)
                                // TODO: when implementing cross platform support, we need to
                                // check the library extension based on the platform (.dll for
                                // Windows, .dylib for macOS, .so for Linux)
                                // Probably would be good idea to have a helper function for that
                                // in FileSystemOps
                                && entry.path.extension().and_then(|ext| ext.to_str()) == Some("so")
                                && let Some(core_name) = entry.path.file_stem()
                                && let Some(core_name) = core_name.to_str()
                                && get_supported_core(core_name).is_some()
                            {
                                Some(core_name.to_string())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<String>>();
                    Ok(cores)
                }
                Err(e) => Err(Error::IoError(format!(
                    "Failed to read libretro core directory: {}",
                    e
                ))),
            }
        } else {
            Err(Error::SettingsError(
                "Libretro core directory is not set".to_string(),
            ))
        }
    }

    pub async fn get_cores_for_system(
        &self,
        system_id: i64,
    ) -> Result<Vec<CoreMappingModel>, Error> {
        let mappings = self
            .repository_manager
            .get_system_libretro_core_repository()
            .get_mappings_for_system(system_id)
            .await?;
        Ok(mappings
            .into_iter()
            .map(|m| CoreMappingModel {
                id: m.id,
                core_name: m.core_name,
            })
            .collect())
    }

    pub async fn get_systems_for_core(
        &self,
        core_name: &str,
    ) -> Result<Vec<SystemCoreMappingModel>, Error> {
        let mappings = self
            .repository_manager
            .get_system_libretro_core_repository()
            .get_mappings_for_core(core_name)
            .await?;
        Ok(mappings
            .into_iter()
            .map(|m| SystemCoreMappingModel {
                id: m.id,
                system_id: m.system_id,
                system_name: m.system_name,
            })
            .collect())
    }

    pub async fn add_core_mapping(&self, system_id: i64, core_name: &str) -> Result<i64, Error> {
        if get_supported_core(core_name).is_none() {
            return Err(Error::InvalidInput(format!(
                "'{}' is not a recognized libretro core",
                core_name
            )));
        }

        let id = self
            .repository_manager
            .get_system_libretro_core_repository()
            .add_mapping(system_id, core_name)
            .await?;
        Ok(id)
    }

    pub async fn remove_core_mapping(&self, mapping_id: i64) -> Result<(), Error> {
        self.repository_manager
            .get_system_libretro_core_repository()
            .remove_mapping(mapping_id)
            .await?;
        Ok(())
    }

    fn get_core_file_name(&self, core_name: &str) -> String {
        // TODO: if implementing cross platform support, we need to check the library extension
        // based on the platform
        format!("{}.so", core_name)
    }

    pub async fn get_core_system_info(
        &self,
        core_name: &str,
    ) -> Result<LibretroCoreInfo, LibretroPreflightError> {
        let libretro_core_dir = self
            .settings
            .libretro_core_dir
            .as_ref()
            .ok_or(LibretroPreflightError::CoreDirNotSet)?;
        let libretro_system_dir = self
            .settings
            .libretro_system_dir
            .as_ref()
            .ok_or(LibretroPreflightError::SystemDirNotSet)?;

        let supported_core = get_supported_core(core_name).ok_or(
            LibretroPreflightError::CoreNotRecognized(core_name.to_string()),
        )?;

        let res = libretro_runner::libretro_info_parser::parse_libretro_info(
            core_name,
            libretro_core_dir.as_ref(),
        )
        .await
        .map_err(|e| LibretroPreflightError::InfoParseError(e.to_string()))?;

        let is_available = self
            .fs_ops
            .is_file(&libretro_core_dir.join(self.get_core_file_name(core_name)));

        tracing::info!("Core '{}' availability: {}", core_name, is_available);

        let firmware: Vec<LibretroFirmwareInfo> = res
            .firmware
            .iter()
            .map(|f| {
                let firmware_path = libretro_system_dir.join(&f.path);
                let available = self.fs_ops.is_file(&firmware_path);
                tracing::info!("Firmware '{}' availability: {}", f.path, available);
                LibretroFirmwareInfo {
                    desc: f.desc.clone(),
                    path: f.path.clone(),
                    opt: f.opt,
                    available,
                }
            })
            .collect();

        Ok(LibretroCoreInfo {
            core_name: res.core_name,
            is_available,
            firmware_info: firmware,
            input_profile: supported_core.input_profile,
            supported_extensions: res.supported_extensions.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::file_system_ops::mock::MockFileSystemOps;

    use super::*;

    async fn make_service(
        settings: Arc<Settings>,
        mock_fs: MockFileSystemOps,
    ) -> LibretroCoreService {
        let pool = Arc::new(database::setup_test_db().await);
        let repo_manager = Arc::new(RepositoryManager::new(pool));
        LibretroCoreService::new(settings, Arc::new(mock_fs), repo_manager)
    }

    #[async_std::test]
    async fn test_list_cores() {
        let settings = Arc::new(Settings {
            libretro_core_dir: Some("/fake/cores".into()),
            ..Default::default()
        });
        let mock_fs_ops = MockFileSystemOps::new();
        mock_fs_ops.add_file("/fake/cores/fake.so");
        mock_fs_ops.add_file("/fake/cores/fceumm_libretro.so");

        let service = make_service(settings, mock_fs_ops).await;
        let cores = service.list_cores().unwrap();
        assert_eq!(cores, vec!["fceumm_libretro".to_string()]);
    }

    #[async_std::test]
    async fn test_add_core_mapping_unsupported_core_rejected() {
        let settings = Arc::new(Settings::default());
        let service = make_service(settings, MockFileSystemOps::new()).await;

        let result = service.add_core_mapping(1, "unsupported_core").await;
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }
}
