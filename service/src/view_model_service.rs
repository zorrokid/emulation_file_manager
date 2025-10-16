use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    view_models::{
        DocumentViewerListModel, DocumentViewerViewModel, EmulatorViewModel, FileInfoViewModel,
        FileSetListModel, FileSetViewModel, ReleaseListModel, ReleaseViewModel, Settings,
        SoftwareTitleListModel, SystemListModel,
    },
};

use core_types::{ArgumentType, FileType};

#[derive(Debug, Clone, Default)]
pub struct ReleaseFilter {
    pub system_id: Option<i64>,
    pub software_title_id: Option<i64>,
    pub file_set_id: Option<i64>,
}

#[derive(Debug)]
pub struct ViewModelService {
    repository_manager: Arc<RepositoryManager>,
}

impl ViewModelService {
    pub fn new(repository_manager: Arc<RepositoryManager>) -> Self {
        Self { repository_manager }
    }

    pub async fn get_emulator_view_model(
        &self,
        emulator_id: i64,
    ) -> Result<EmulatorViewModel, Error> {
        let emulator = self
            .repository_manager
            .get_emulator_repository()
            .get_emulator(emulator_id)
            .await?;

        let system = self
            .repository_manager
            .get_system_repository()
            .get_system(emulator.system_id)
            .await?;

        let system = SystemListModel {
            id: system.id,
            name: system.name,
            can_delete: false,
        };

        let arguments: Vec<ArgumentType> =
            serde_json::from_str(&emulator.arguments).map_err(|_| {
                Error::DeserializationError(format!(
                    "Invalid argument format for emulator {}: {}",
                    emulator.name, emulator.arguments
                ))
            })?;

        Ok(EmulatorViewModel {
            id: emulator.id,
            name: emulator.name,
            executable: emulator.executable,
            extract_files: emulator.extract_files,
            arguments,
            system,
        })
    }

    pub async fn get_document_viewer_list_models(
        &self,
    ) -> Result<Vec<DocumentViewerListModel>, Error> {
        let document_viewers = self
            .repository_manager
            .get_document_viewer_repository()
            .get_document_viewers()
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let list_models: Vec<DocumentViewerListModel> = document_viewers
            .iter()
            .map(DocumentViewerListModel::from)
            .collect();

        Ok(list_models)
    }

    pub async fn get_emulator_view_models_for_systems(
        &self,
        system_ids: &[i64],
    ) -> Result<Vec<EmulatorViewModel>, Error> {
        let emulators = self
            .repository_manager
            .get_emulator_repository()
            .get_emulators_for_systems(system_ids)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let mut emulator_view_models: Vec<EmulatorViewModel> = vec![];

        for emulator in emulators {
            let system = self
                .repository_manager
                .get_system_repository()
                .get_system(emulator.system_id)
                .await
                .map_err(|err| Error::DbError(err.to_string()))?;

            let system = SystemListModel {
                id: system.id,
                name: system.name,
                can_delete: false,
            };

            let arguments: Vec<ArgumentType> =
                serde_json::from_str(&emulator.arguments).map_err(|_| {
                    Error::DeserializationError(format!(
                        "Invalid argument format for emulator {}: {}",
                        emulator.name, emulator.arguments
                    ))
                })?;

            let view_model = EmulatorViewModel {
                id: emulator.id,
                name: emulator.name,
                executable: emulator.executable,
                extract_files: emulator.extract_files,
                system,
                arguments,
            };

            emulator_view_models.push(view_model);
        }

        Ok(emulator_view_models)
    }

    pub async fn get_document_viewer_view_models(
        &self,
    ) -> Result<Vec<DocumentViewerViewModel>, Error> {
        let document_viewers = self
            .repository_manager
            .get_document_viewer_repository()
            .get_document_viewers()
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let mut view_models: Vec<DocumentViewerViewModel> = vec![];

        for document_viewer in document_viewers {
            let arguments: Vec<ArgumentType> = serde_json::from_str(&document_viewer.arguments)
                .map_err(|_| {
                    Error::DeserializationError(format!(
                        "Invalid argument format for document viewer {}: {}",
                        document_viewer.name, document_viewer.arguments
                    ))
                })?;

            let view_model = DocumentViewerViewModel {
                id: document_viewer.id,
                name: document_viewer.name,
                executable: document_viewer.executable,
                arguments,
                document_type: document_viewer.document_type,
            };

            view_models.push(view_model);
        }

        Ok(view_models)
    }

    pub async fn get_settings(&self) -> Result<Settings, Error> {
        let settings_map = self
            .repository_manager
            .get_settings_repository()
            .get_settings()
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;
        Ok(Settings::from(settings_map))
    }

    pub async fn get_system_list_models(&self) -> Result<Vec<SystemListModel>, Error> {
        let systems = self
            .repository_manager
            .get_system_repository()
            .get_systems()
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let mut list_models: Vec<SystemListModel> =
            systems.iter().map(SystemListModel::from).collect();

        for system in list_models.iter_mut() {
            system.can_delete = !self
                .repository_manager
                .get_system_repository()
                .is_system_in_use(system.id)
                .await
                .map_err(|err| Error::DbError(err.to_string()))?;
        }

        Ok(list_models)
    }

    pub async fn get_software_title_list_models(
        &self,
    ) -> Result<Vec<SoftwareTitleListModel>, Error> {
        let systems = self
            .repository_manager
            .get_software_title_repository()
            // TODO: add search filter
            .get_all_software_titles()
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let mut list_models: Vec<SoftwareTitleListModel> =
            systems.iter().map(SoftwareTitleListModel::from).collect();

        for system in list_models.iter_mut() {
            system.can_delete = !self
                .repository_manager
                .get_software_title_repository()
                .is_software_title_in_use(system.id)
                .await
                .map_err(|err| Error::DbError(err.to_string()))?;
        }

        Ok(list_models)
    }

    pub async fn get_file_set_list_models(
        &self,
        file_type: FileType,
        system_ids: &[i64],
    ) -> Result<Vec<FileSetListModel>, Error> {
        let file_sets = self
            .repository_manager
            .get_file_set_repository()
            .get_file_sets_by_file_type_and_systems(file_type, system_ids)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let list_models: Vec<FileSetListModel> =
            file_sets.iter().map(FileSetListModel::from).collect();

        Ok(list_models)
    }

    pub async fn get_all_file_set_list_models(&self) -> Result<Vec<FileSetListModel>, Error> {
        let file_sets = self
            .repository_manager
            .get_file_set_repository()
            // TODO: get filtered subset of file sets
            .get_all_file_sets()
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let list_models: Vec<FileSetListModel> =
            file_sets.iter().map(FileSetListModel::from).collect();

        Ok(list_models)
    }

    pub async fn get_systems_for_file_set(
        &self,
        file_set_id: i64,
    ) -> Result<Vec<SystemListModel>, Error> {
        let systems = self
            .repository_manager
            .get_system_repository()
            .get_systems_by_file_set(file_set_id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let list_models: Vec<SystemListModel> = systems.iter().map(SystemListModel::from).collect();

        Ok(list_models)
    }

    pub async fn get_release_list_models(
        &self,
        filters: ReleaseFilter,
    ) -> Result<Vec<ReleaseListModel>, Error> {
        let releases = self
            .repository_manager
            .get_release_repository()
            .get_releases(
                filters.system_id,
                filters.software_title_id,
                filters.file_set_id,
            )
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let release_models = releases.iter().map(ReleaseListModel::from).collect();
        Ok(release_models)
    }

    pub async fn get_release_view_model(&self, release_id: i64) -> Result<ReleaseViewModel, Error> {
        let release = self
            .repository_manager
            .get_release_repository()
            .get_release(release_id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let software_titles = self
            .repository_manager
            .get_software_title_repository()
            .get_software_titles_by_release(release_id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let systems = self
            .repository_manager
            .get_system_repository()
            .get_systems_by_release(release_id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let file_sets = self
            .repository_manager
            .get_file_set_repository()
            .get_file_sets_by_release(release_id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let mut file_set_view_models: Vec<FileSetViewModel> = vec![];

        for file_set in file_sets {
            let files = self
                .repository_manager
                .get_file_set_repository()
                .get_file_set_file_info(file_set.id)
                .await
                .map_err(|err| Error::DbError(err.to_string()))?;

            file_set_view_models.push(FileSetViewModel {
                id: file_set.id,
                file_set_name: file_set.name.clone(),
                file_type: file_set.file_type,
                files,
                file_name: file_set.file_name.clone(),
                source: file_set.source.clone(),
            });
        }

        let release_view_model = ReleaseViewModel {
            id: release.id,
            name: release.name.clone(),
            systems,
            software_titles,
            file_sets: file_set_view_models,
        };

        Ok(release_view_model)
    }

    pub async fn get_file_set_view_model(
        &self,
        file_set_id: i64,
    ) -> Result<FileSetViewModel, Error> {
        let file_set = self
            .repository_manager
            .get_file_set_repository()
            .get_file_set(file_set_id)
            .await?;

        let files = self
            .repository_manager
            .get_file_set_repository()
            /*
                         *    Key refactoring recommendations:
                 - Extract file system operations - Move std::fs::remove_file behind a trait to enable mocking in tests.
                 - Extract the path construction logic - The PathBuf::from(&collection_root_dir).join(...).join(...) could be a separate method.
                 - Break down the monolithic method - The delete_file_set method does too much (checking usage, collecting files, syncing, deleting). Split it into smaller, testable functions like collect_deletable_files,
               mark_for_cloud_deletion, and delete_local_file.
                 - Better error handling - Line 95-98 uses eprintln! and swallows errors. Consider collecting errors and returning them, or using proper logging.
                 - Remove TODOs before production - Lines 101-103 and 116-117 have TODOs about ensuring cascade deletions work. These should be verified and the comments removed, or the logic should be implemented.
                 - Inconsistent error mapping - Sometimes you use ? operator (line 28), sometimes .map_err(|e| Error::DbError(e.to_string()))? (line 42). Consider implementing From trait for consistent error conversion.
                 - Magic slice pattern - Line 57 uses if let [entry] = &res[..] which is correct but could be clearer with a comment or extracted to a named function like is_only_in_file_set.
                 - Separate concerns - The cloud sync logic (lines 68-85) could be its own method to simplify testing and improve readability.
            */
            .get_file_set_file_info(file_set.id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        Ok(FileSetViewModel {
            id: file_set.id,
            file_set_name: file_set.name.clone(),
            file_type: file_set.file_type,
            files,
            file_name: file_set.file_name.clone(),
            source: file_set.source.clone(),
        })
    }

    pub async fn get_file_info_view_model(
        &self,
        file_info_id: i64,
    ) -> Result<FileInfoViewModel, Error> {
        let file_info = self
            .repository_manager
            .get_file_info_repository()
            .get_file_info(file_info_id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let file_sets = self
            .repository_manager
            .get_file_set_repository()
            .get_file_sets_by_file_info(file_info_id)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let list_models: Vec<FileSetListModel> =
            file_sets.iter().map(FileSetListModel::from).collect();

        let view_model = FileInfoViewModel {
            id: file_info.id,
            sha1_checksum: file_info.sha1_checksum,
            file_size: file_info.file_size,
            archive_file_name: file_info.archive_file_name,
            belongs_to_file_sets: list_models,
        };
        Ok(view_model)
    }
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use super::*;
    use core_types::SettingName;
    use database::setup_test_db;

    #[async_std::test]
    async fn test_get_emulator_view_model() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let repository_manager = Arc::new(RepositoryManager::new(pool.clone()));
        let view_model_service = ViewModelService::new(repository_manager.clone());
        let system_id = repository_manager
            .get_system_repository()
            .add_system(&"Test System".to_string())
            .await
            .unwrap();

        let emulator_id = repository_manager
            .get_emulator_repository()
            .add_emulator(
                &"Test Emulator".to_string(),
                &"temu".to_string(),
                false,
                &[ArgumentType::Flag {
                    name: "args".into(),
                }],
                system_id,
            )
            .await
            .unwrap();

        let emulator_view_model = view_model_service
            .get_emulator_view_model(emulator_id)
            .await
            .unwrap();

        assert_eq!(emulator_view_model.id, emulator_id);
        assert_eq!(emulator_view_model.name, "Test Emulator");
        assert_eq!(emulator_view_model.executable, "temu");
        assert_eq!(
            emulator_view_model.arguments,
            vec![ArgumentType::Flag {
                name: "args".into(),
            }]
        );
        assert!(!emulator_view_model.extract_files);
        assert_eq!(emulator_view_model.system.id, system_id);
        assert_eq!(emulator_view_model.system.name, "Test System");
    }

    #[async_std::test]
    async fn test_get_settings() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let repository_manager = Arc::new(RepositoryManager::new(pool.clone()));
        let view_model_service = ViewModelService::new(repository_manager.clone());

        repository_manager
            .get_settings_repository()
            .add_setting(&SettingName::CollectionRootDir, "test_value")
            .await
            .unwrap();

        let settings = view_model_service.get_settings().await.unwrap();
        assert_eq!(settings.collection_root_dir, PathBuf::from("test_value"));
    }
}
