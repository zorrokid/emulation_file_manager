use std::{
    cell::OnceCell,
    path::{Path, PathBuf},
    sync::Arc,
};

use core_types::Sha1Checksum;
use database::models::{FileSetFileInfo, System};
use emulator_runner::{error::EmulatorRunnerError, run_with_emulator};
use file_export::{export_files, export_files_zipped, FileExportError};
use iced::{
    widget::{button, column, pick_list, text},
    Element, Task,
};
use service::{
    error::Error,
    view_model_service::ViewModelService,
    view_models::{EmulatorViewModel, FileSetViewModel, ReleaseViewModel, Settings},
};

pub struct ReleaseViewWidget {
    release: Option<ReleaseViewModel>,
    selected_file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfo>,
    selected_emulator: Option<EmulatorViewModel>,
    view_model_service: Arc<ViewModelService>,
    emulators: Vec<EmulatorViewModel>,
    selected_system: Option<System>,
    collection_root_dir: OnceCell<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum ReleaseViewWidgetMessage {
    SetEditRelease(ReleaseViewModel),
    SetRelease(ReleaseViewModel),
    // Local messages
    StartEditRelease,
    SetSelectedFileSet(FileSetViewModel),
    SetSelectedFile(FileSetFileInfo),
    RunWithEmulator,
    EmulatorsLoaded(Result<Vec<EmulatorViewModel>, Error>),
    SetSelectedEmulator(EmulatorViewModel),
    FinishedRunWithEmulator(Result<(), EmulatorRunnerError>),
    SetSelectedSystem(System),
    SettingsFetched(Result<Settings, Error>),
    FilesExported(Result<(), FileExportError>),
}

impl ReleaseViewWidget {
    pub fn new(
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<ReleaseViewWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_settings_task = Task::perform(
            async move { view_model_service_clone.get_settings().await },
            ReleaseViewWidgetMessage::SettingsFetched,
        );

        (
            ReleaseViewWidget {
                release: None,
                selected_file_set: None,
                selected_file: None,
                selected_emulator: None,
                view_model_service,
                emulators: vec![],
                selected_system: None,
                collection_root_dir: OnceCell::new(),
            },
            fetch_settings_task,
        )
    }

    pub fn update(&mut self, message: ReleaseViewWidgetMessage) -> Task<ReleaseViewWidgetMessage> {
        match message {
            ReleaseViewWidgetMessage::StartEditRelease => {
                if let Some(release) = &self.release {
                    Task::done(ReleaseViewWidgetMessage::SetEditRelease(release.clone()))
                } else {
                    Task::none()
                }
            }
            ReleaseViewWidgetMessage::SetRelease(release) => {
                let system_ids = release.systems.iter().map(|s| s.id).collect::<Vec<_>>();
                self.release = Some(release);
                self.selected_file_set = None;
                self.selected_file = None;
                let view_model_service = Arc::clone(&self.view_model_service);
                Task::perform(
                    async move {
                        view_model_service
                            .get_emulator_view_models_for_systems(system_ids)
                            .await
                    },
                    ReleaseViewWidgetMessage::EmulatorsLoaded,
                )
            }
            ReleaseViewWidgetMessage::SetSelectedFileSet(file_set) => {
                self.selected_file_set = Some(file_set);
                self.selected_file = None;
                Task::none()
            }
            ReleaseViewWidgetMessage::SetSelectedFile(file) => {
                self.selected_file = Some(file);
                Task::none()
            }
            ReleaseViewWidgetMessage::EmulatorsLoaded(result) => match result {
                Ok(emulators) => {
                    self.emulators = emulators;
                    Task::none()
                }
                Err(err) => {
                    // Handle error, e.g., show a notification
                    eprintln!("Error loading emulators: {}", err);
                    Task::none()
                }
            },
            ReleaseViewWidgetMessage::RunWithEmulator => {
                // TODO: first need to export files using export_files or export_files_zipped
                // depending on the emulator extract setting
                if let (Some(file), Some(file_set), Some(emulator), Some(system), Some(root_dir)) = (
                    &self.selected_file,
                    &self.selected_file_set,
                    &self.selected_emulator,
                    &self.selected_system,
                    &self.collection_root_dir.get(),
                ) {
                    let emulator_system = emulator.systems.iter().find(|s| s.id == system.id);
                    if let Some(system) = emulator_system {
                        let executable = emulator.executable.clone();
                        let arguments = system.arguments.clone();
                        let files = file_set
                            .files
                            .iter()
                            .map(|f| f.file_name.clone())
                            .collect::<Vec<_>>();
                        let selected_file = file.file_name.clone();

                        let root_dir = PathBuf::from(root_dir);
                        let temp_dir = std::env::temp_dir();
                        let root_path = PathBuf::from(&root_dir);
                        let temp_path = PathBuf::from(&temp_dir);
                        let output_file_name_mapping = file_set
                            .files
                            .iter()
                            .map(|f| (f.archive_file_name.clone(), f.file_name.clone()))
                            .collect::<std::collections::HashMap<_, _>>();

                        let filename_checksum_mapping = file_set
                            .files
                            .iter()
                            .map(|f| {
                                let checksum: Sha1Checksum = f
                                    .sha1_checksum
                                    .clone()
                                    .try_into()
                                    .expect("Failed to convert to Sha1Checksum");
                                (f.archive_file_name.clone(), checksum)
                            })
                            .collect::<std::collections::HashMap<String, Sha1Checksum>>();
                        let exported_zip_file_name = file_set.file_set_name.clone();

                        let extract_files = emulator.extract_files;
                        Task::perform(
                            async move {
                                let result = if extract_files {
                                    export_files(
                                        root_path,
                                        temp_path.clone(),
                                        output_file_name_mapping,
                                        filename_checksum_mapping,
                                    )
                                } else {
                                    export_files_zipped(
                                        root_path,
                                        temp_path.clone(),
                                        output_file_name_mapping,
                                        filename_checksum_mapping,
                                        exported_zip_file_name,
                                    )
                                };
                                match result {
                                    Ok(_) => {
                                        // If export was successful, run the emulator
                                        run_with_emulator(
                                            executable,
                                            arguments,
                                            files,
                                            selected_file,
                                            temp_path,
                                        )
                                        .await
                                    }
                                    Err(err) => Err(EmulatorRunnerError::IoError(format!(
                                        "Failed to export files: {}",
                                        err
                                    ))),
                                }
                            },
                            ReleaseViewWidgetMessage::FinishedRunWithEmulator,
                        )
                    } else {
                        // Handle case where system is not found in emulator's systems
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            ReleaseViewWidgetMessage::SetSelectedEmulator(emulator) => {
                self.selected_emulator = Some(emulator);
                Task::none()
            }
            ReleaseViewWidgetMessage::SetSelectedSystem(system) => {
                self.selected_system = Some(system);
                Task::none()
            }
            ReleaseViewWidgetMessage::SettingsFetched(result) => match result {
                Ok(settings) => {
                    self.collection_root_dir
                        .set(settings.collection_root_dir.clone())
                        .expect("Failed to set collection root dir");
                    Task::none()
                }
                Err(err) => {
                    // Handle error, e.g., show a notification
                    eprintln!("Error fetching settings: {}", err);
                    Task::none()
                }
            },
            ReleaseViewWidgetMessage::FinishedRunWithEmulator(result) => match result {
                Ok(_) => {
                    // TODO: clean exported files
                    Task::none()
                }
                Err(err) => {
                    // Handle error, e.g., show a notification
                    eprintln!("Error running emulator: {}", err);
                    Task::none()
                }
            },
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<ReleaseViewWidgetMessage> {
        if let Some(release) = &self.release {
            let release_name_field = text!("Release Name: {}", release.name);
            let software_titles_field = text!("Software Titles: {:?}", release.software_titles);
            let system_names_field = text!("Systems: {:?}", release.systems);
            let edit_button = button("Edit").on_press(ReleaseViewWidgetMessage::StartEditRelease);
            let file_sets_select: Element<ReleaseViewWidgetMessage> = pick_list(
                release.file_sets.as_slice(),
                self.selected_file_set.clone(),
                ReleaseViewWidgetMessage::SetSelectedFileSet,
            )
            .into();

            let file_select: Element<ReleaseViewWidgetMessage> =
                if let Some(selected_file_set) = &self.selected_file_set {
                    pick_list(
                        selected_file_set.files.as_slice(),
                        self.selected_file.clone(),
                        ReleaseViewWidgetMessage::SetSelectedFile,
                    )
                    .into()
                } else {
                    text("No file set selected").into()
                };

            let emulator_select: Element<ReleaseViewWidgetMessage> = pick_list(
                self.emulators.as_slice(),
                self.selected_emulator.clone(),
                ReleaseViewWidgetMessage::SetSelectedEmulator,
            )
            .into();

            let system_select: Element<ReleaseViewWidgetMessage> = pick_list(
                release.systems.as_slice(),
                self.selected_system.clone(),
                ReleaseViewWidgetMessage::SetSelectedSystem,
            )
            .into();

            let run_with_emulator_button = button("Run with Emulator").on_press_maybe(
                (self.selected_file.is_some() && self.selected_file_set.is_some())
                    .then_some(ReleaseViewWidgetMessage::RunWithEmulator),
            );

            column![
                release_name_field,
                software_titles_field,
                system_names_field,
                edit_button,
                file_sets_select,
                file_select,
                system_select,
                emulator_select,
                run_with_emulator_button,
            ]
            .into()
        } else {
            text("No release selected").into()
        }
    }
}
