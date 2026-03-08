use std::sync::Arc;

use crate::{error::Error, file_system_ops::FileSystemOps, view_models::Settings};

pub struct LibretroCoreService {
    pub settings: Arc<Settings>,
    pub fs_ops: Arc<dyn FileSystemOps>,
    pub supported_cores: Vec<String>,
}

impl std::fmt::Debug for LibretroCoreService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibretroCoreService")
            .field("settings", &"Settings { ... }")
            .field("fs_ops", &"FileSystemOps { ... }")
            .field("supported_cores", &self.supported_cores)
            .finish()
    }
}

impl LibretroCoreService {
    pub fn new(
        settings: Arc<Settings>,
        fs_ops: Arc<dyn FileSystemOps>,
        supported_cores: Vec<String>,
    ) -> Self {
        Self {
            settings,
            fs_ops,
            supported_cores,
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
                                && self.fs_ops.is_file(&entry.path)
                                // TODO: when implementing cross platform support, we need to
                                // check the library extension based on the platform (.dll for
                                // Windows, .dylib for macOS, .so for Linux)
                                // Probably would be good idea to have a helper function for that
                                // in FileSystemOps
                                && entry.path.extension().and_then(|ext| ext.to_str()) == Some("so")
                                && let Some(file_name) = entry.path.file_stem()
                                && let Some(file_name) = file_name.to_str()
                                && self.supported_cores.contains(&file_name.to_string())
                            {
                                Some(file_name.to_string())
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
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use crate::file_system_ops::{SimpleDirEntry, mock::MockFileSystemOps};

    use super::*;

    #[test]
    fn test_list_cores() {
        let settings = Arc::new(Settings {
            libretro_core_dir: Some("/fake/cores".into()),
            ..Default::default()
        });
        let mock_fs_ops = MockFileSystemOps::new();

        let supported_core_path = "/fake/cores/lib_supported.so";
        let non_supported_core_path = "/fake/cores/lib_unsupported.so";
        let supported_cores = vec!["lib_supported".to_string()];

        // Add files to mock (both supported and unsupported cores)
        mock_fs_ops.add_file(supported_core_path);
        mock_fs_ops.add_file(non_supported_core_path);

        let service = LibretroCoreService::new(settings, Arc::new(mock_fs_ops), supported_cores);
        let cores = service.list_cores().unwrap();
        assert_eq!(cores, vec!["lib_supported".to_string()]);
    }
}
