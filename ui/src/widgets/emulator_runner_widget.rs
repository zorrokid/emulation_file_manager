use std::{cell::OnceCell, collections::HashMap, path::PathBuf, sync::Arc};

use core_types::Sha1Checksum;
use database::models::{FileSetFileInfo, System};
use emulator_runner::{error::EmulatorRunnerError, run_with_emulator};
use file_export::{export_files, export_files_zipped};
use iced::{
    widget::{button, column, pick_list, row, text},
    Element, Task,
};
use service::{
    error::Error,
    view_model_service::ViewModelService,
    view_models::{EmulatorViewModel, FileSetViewModel, ReleaseViewModel, Settings},
};

use crate::{
    defaults::{DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_SPACING},
    util::file_paths::resolve_file_type_path,
};

pub struct EmulatorRunnerWidget {
    selected_file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfo>,
    selected_emulator: Option<EmulatorViewModel>,
    emulators: Vec<EmulatorViewModel>,
    selected_system: Option<System>,
    collection_root_dir: OnceCell<PathBuf>,
    view_model_service: Arc<ViewModelService>,
    file_sets: Vec<FileSetViewModel>,
    systems: Vec<System>,
}

#[derive(Debug, Clone)]
pub enum EmulatorRunnerWidgetMessage {
    Reset,
    SettingsFetched(Result<Settings, Error>),
    SetSelectedFileSet(FileSetViewModel),
    SetSelectedFile(FileSetFileInfo),
    RunWithEmulator,
    EmulatorsLoaded(Result<Vec<EmulatorViewModel>, Error>),
    SetSelectedEmulator(EmulatorViewModel),
    FinishedRunWithEmulator(Result<(), EmulatorRunnerError>),
    SetSelectedSystem(System),
    ReleaseChanged(ReleaseViewModel),
}

impl EmulatorRunnerWidget {
    pub fn new(
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<EmulatorRunnerWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_settings_task = Task::perform(
            async move { view_model_service_clone.get_settings().await },
            EmulatorRunnerWidgetMessage::SettingsFetched,
        );

        (
            EmulatorRunnerWidget {
                selected_file_set: None,
                selected_file: None,
                selected_emulator: None,
                emulators: vec![],
                selected_system: None,
                collection_root_dir: OnceCell::new(),
                view_model_service,
                systems: vec![],
                file_sets: vec![],
            },
            fetch_settings_task,
        )
    }

    pub fn update(
        &mut self,
        message: EmulatorRunnerWidgetMessage,
    ) -> Task<EmulatorRunnerWidgetMessage> {
        match message {
            EmulatorRunnerWidgetMessage::SettingsFetched(result) => match result {
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
            EmulatorRunnerWidgetMessage::ReleaseChanged(release) => {
                let system_ids = release.systems.iter().map(|s| s.id).collect::<Vec<_>>();
                self.reset();
                self.systems = release.systems.clone();
                if self.systems.len() == 1 {
                    self.selected_system = self.systems.first().cloned();
                }
                self.file_sets = release.file_sets.clone();
                if self.file_sets.len() == 1 {
                    self.selected_file_set = self.file_sets.first().cloned();
                    if let Some(file_set) = &self.selected_file_set {
                        if file_set.files.len() == 1 {
                            self.selected_file = file_set.files.first().cloned();
                        }
                    }
                }
                let view_model_service = Arc::clone(&self.view_model_service);
                Task::perform(
                    async move {
                        view_model_service
                            .get_emulator_view_models_for_systems(system_ids)
                            .await
                    },
                    EmulatorRunnerWidgetMessage::EmulatorsLoaded,
                )
            }
            EmulatorRunnerWidgetMessage::SetSelectedFileSet(file_set) => {
                self.selected_file_set = Some(file_set);
                self.selected_file = None;
                Task::none()
            }
            EmulatorRunnerWidgetMessage::SetSelectedFile(file) => {
                self.selected_file = Some(file);
                Task::none()
            }
            EmulatorRunnerWidgetMessage::EmulatorsLoaded(result) => match result {
                Ok(emulators) => {
                    self.emulators = emulators;
                    if self.emulators.len() == 1 {
                        self.selected_emulator = self.emulators.first().cloned();
                    } else {
                        self.selected_emulator = None;
                    }
                    Task::none()
                }
                Err(err) => {
                    // Handle error, e.g., show a notification
                    eprintln!("Error loading emulators: {}", err);
                    Task::none()
                }
            },
            EmulatorRunnerWidgetMessage::RunWithEmulator => {
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
                        let file_set_name = file_set.file_set_name.clone();

                        let root_dir = PathBuf::from(root_dir);
                        let temp_dir = std::env::temp_dir();
                        let source_path = resolve_file_type_path(&root_dir, &file_set.file_type);
                        let temp_path = PathBuf::from(&temp_dir);
                        let output_file_name_mapping = file_set
                            .files
                            .iter()
                            .map(|f| (f.archive_file_name.clone(), f.file_name.clone()))
                            .collect::<HashMap<_, _>>();

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
                            .collect::<HashMap<String, Sha1Checksum>>();
                        let exported_zip_file_name = file_set.file_set_name.clone();

                        let extract_files = emulator.extract_files;
                        let starting_file = if extract_files {
                            file.file_name.clone()
                        } else {
                            exported_zip_file_name.clone()
                        };
                        Task::perform(
                            async move {
                                let result = if extract_files {
                                    export_files(
                                        source_path,
                                        temp_path.clone(),
                                        output_file_name_mapping,
                                        filename_checksum_mapping,
                                    )
                                } else {
                                    export_files_zipped(
                                        source_path,
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
                                            starting_file,
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
                            EmulatorRunnerWidgetMessage::FinishedRunWithEmulator,
                        )
                    } else {
                        // Handle case where system is not found in emulator's systems
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
            EmulatorRunnerWidgetMessage::SetSelectedEmulator(emulator) => {
                self.selected_emulator = Some(emulator);
                Task::none()
            }
            EmulatorRunnerWidgetMessage::SetSelectedSystem(system) => {
                self.selected_system = Some(system);
                Task::none()
            }
            EmulatorRunnerWidgetMessage::FinishedRunWithEmulator(result) => match result {
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
            EmulatorRunnerWidgetMessage::Reset => {
                self.reset();
                Task::none()
            }
        }
    }

    fn reset(&mut self) {
        self.selected_file_set = None;
        self.selected_file = None;
        self.selected_emulator = None;
        self.selected_system = None;
    }

    pub fn view(&self) -> Element<EmulatorRunnerWidgetMessage> {
        let file_set_select_row = row![
            text("File Sets:").width(DEFAULT_LABEL_WIDTH),
            pick_list(
                self.file_sets.as_slice(),
                self.selected_file_set.clone(),
                EmulatorRunnerWidgetMessage::SetSelectedFileSet,
            )
        ];

        let file_select_row = row![
            text("Files:").width(DEFAULT_LABEL_WIDTH),
            pick_list(
                self.selected_file_set
                    .as_ref()
                    .map_or_else(Vec::new, |fs| fs.files.clone()),
                self.selected_file.clone(),
                EmulatorRunnerWidgetMessage::SetSelectedFile,
            )
        ];

        let system_select_row = row![
            text("Systems:").width(DEFAULT_LABEL_WIDTH),
            pick_list(
                self.systems.as_slice(),
                self.selected_system.clone(),
                EmulatorRunnerWidgetMessage::SetSelectedSystem,
            )
        ];

        let emulator_select_row = row![
            text("Emulators:").width(DEFAULT_LABEL_WIDTH),
            pick_list(
                self.emulators.as_slice(),
                self.selected_emulator.clone(),
                EmulatorRunnerWidgetMessage::SetSelectedEmulator,
            ),
            button("Run with Emulator").on_press_maybe(
                (self.selected_file.is_some() && self.selected_file_set.is_some())
                    .then_some(EmulatorRunnerWidgetMessage::RunWithEmulator),
            )
        ];

        column![
            file_set_select_row,
            file_select_row,
            system_select_row,
            emulator_select_row,
        ]
        .padding(DEFAULT_PADDING)
        .spacing(DEFAULT_SPACING)
        .into()
    }
}
