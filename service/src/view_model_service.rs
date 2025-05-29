use std::sync::Arc;

use database::repository_manager::RepositoryManager;

use crate::{
    error::Error,
    view_models::{
        EmulatorListModel, EmulatorSystemViewModel, EmulatorViewModel, FileSetListModel,
        FileSetViewModel, ReleaseListModel, ReleaseViewModel, Settings, SoftwareTitleListModel,
        SystemListModel,
    },
};

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
        let (emulator, emulator_systems) = self
            .repository_manager
            .get_emulator_repository()
            .get_emulator_with_systems(emulator_id)
            .await?;

        println!("Emulator: {:?}", emulator);
        println!("Emulator Systems: {:?}", emulator_systems);

        Ok(EmulatorViewModel {
            id: emulator.id,
            name: emulator.name,
            executable: emulator.executable,
            extract_files: emulator.extract_files,
            systems: emulator_systems
                .into_iter()
                .map(|es| EmulatorSystemViewModel {
                    id: es.id,
                    system_id: es.system_id,
                    system_name: es.system_name,
                    arguments: es.arguments,
                })
                .collect(),
        })
    }

    pub async fn get_emulator_list_models(&self) -> Result<Vec<EmulatorListModel>, Error> {
        let emulators = self
            .repository_manager
            .get_emulator_repository()
            .get_emulators()
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let list_models: Vec<EmulatorListModel> =
            emulators.iter().map(EmulatorListModel::from).collect();

        Ok(list_models)
    }

    pub async fn get_emulator_view_models_for_systems(
        &self,
        system_ids: Vec<i64>,
    ) -> Result<Vec<EmulatorViewModel>, Error> {
        let emulators = self
            .repository_manager
            .get_emulator_repository()
            .get_emulators_for_systems(system_ids)
            .await
            .map_err(|err| Error::DbError(err.to_string()))?;

        let mut emulator_view_models: Vec<EmulatorViewModel> = vec![];

        for emulator in emulators {
            let (emulator, emulator_systems) = self
                .repository_manager
                .get_emulator_repository()
                .get_emulator_with_systems(emulator.id)
                .await
                .map_err(|err| Error::DbError(err.to_string()))?;

            let view_model = EmulatorViewModel {
                id: emulator.id,
                name: emulator.name,
                executable: emulator.executable,
                extract_files: emulator.extract_files,
                systems: emulator_systems
                    .into_iter()
                    .map(|es| EmulatorSystemViewModel {
                        id: es.id,
                        system_id: es.system_id,
                        system_name: es.system_name,
                        arguments: es.arguments,
                    })
                    .collect(),
            };

            emulator_view_models.push(view_model);
        }

        Ok(emulator_view_models)
    }

    pub async fn get_settings(&self) -> Result<Settings, Error> {
        let settings_map = self
            .repository_manager
            .settings()
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

    pub async fn get_file_set_list_models(&self) -> Result<Vec<FileSetListModel>, Error> {
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

    pub async fn get_release_list_models(&self) -> Result<Vec<ReleaseListModel>, Error> {
        let releases = self
            .repository_manager
            .get_release_repository()
            .get_releases()
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
                file_set_name: file_set.file_name.clone(),
                file_type: file_set.file_type,
                files,
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
}

#[cfg(test)]
mod tests {

    use std::path::PathBuf;

    use super::*;
    use database::{
        models::{EmulatorSystemUpdateModel, SettingName},
        setup_test_db,
    };

    #[async_std::test]
    async fn test_get_emulator_view_model() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let repository_manager = Arc::new(RepositoryManager::new(pool.clone()));
        let view_model_service = ViewModelService::new(repository_manager.clone());
        let system_id = repository_manager
            .get_system_repository()
            .add_system("Test System".to_string())
            .await
            .unwrap();
        let emulator_systems = vec![EmulatorSystemUpdateModel {
            id: None,
            system_id,
            arguments: "args".to_string(),
        }];

        let emulator_id = repository_manager
            .get_emulator_repository()
            .add_emulator_with_systems(
                "Test Emulator".to_string(),
                "temu".to_string(),
                false,
                emulator_systems,
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
        assert!(!emulator_view_model.extract_files);
        assert_eq!(emulator_view_model.systems.len(), 1);
        assert_eq!(emulator_view_model.systems[0].system_id, system_id);
        assert_eq!(emulator_view_model.systems[0].system_name, "Test System");
        assert_eq!(emulator_view_model.systems[0].arguments, "args");
    }

    #[async_std::test]
    async fn test_get_settings() {
        let pool = setup_test_db().await;
        let pool = Arc::new(pool);
        let repository_manager = Arc::new(RepositoryManager::new(pool.clone()));
        let view_model_service = ViewModelService::new(repository_manager.clone());

        repository_manager
            .settings()
            .add_setting(SettingName::CollectionRootDir.as_str(), "test_value")
            .await
            .unwrap();

        let settings = view_model_service.get_settings().await.unwrap();
        assert_eq!(settings.collection_root_dir, PathBuf::from("test_value"));
    }
}
