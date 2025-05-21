use std::sync::Arc;

use database::{
    database_error::Error, models::EmulatorSystemUpdateModel, repository_manager::RepositoryManager,
};
use iced::{
    widget::{button, checkbox, container, row, text, text_input, Column, Container},
    Element, Task,
};
use service::{
    error::Error as ServiceError,
    view_model_service::ViewModelService,
    view_models::{EmulatorListModel, EmulatorViewModel},
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::emulator_systems_add_widget::{
    self, EmulatorSystemsAddWidget, EmulatorSystemsAddWidgetMessage,
};

#[derive(Debug, Clone)]
pub struct EmulatorSystem {
    pub id: Option<i64>,
    pub system_id: i64,
    pub system_name: String,
    pub arguments: String,
}

pub struct EmulatorAddWidget {
    emulator_name: String,
    emulator_executable: String,
    emulator_extract_files: bool,
    repositories: Arc<RepositoryManager>,
    emulator_systems_widget: EmulatorSystemsAddWidget,
    emulator_id: Option<i64>,
    emulator_systems: Vec<EmulatorSystem>,
    is_adding_emulator: bool,
    view_model_service: Arc<ViewModelService>,
}

#[derive(Debug, Clone)]
pub enum EmulatorAddWidgetMessage {
    // child messages
    EmulatorSystemsAddWidget(EmulatorSystemsAddWidgetMessage),
    // local messages
    EmulatorNameChanged(String),
    EmulatorExecutableChanged(String),
    EmulatorExtractFilesChanged(bool),
    Submit,
    EmulatorSaved(Result<i64, Error>),
    EmulatorAdded(EmulatorListModel),
    EmulatorSystemSaved(Result<i64, Error>),
    StartAddEmulator,
    CancelAddEmulator,
    SetEmulatorId(i64),
    EmulatorFetched(Result<EmulatorViewModel, ServiceError>),
    EditEmulatorSystem(EmulatorSystem),
}

impl EmulatorAddWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<EmulatorAddWidgetMessage>) {
        let (emulator_systems_widget, task) = EmulatorSystemsAddWidget::new(
            Arc::clone(&repositories),
            Arc::clone(&view_model_service),
        );
        (
            Self {
                emulator_name: String::new(),
                emulator_executable: String::new(),
                emulator_extract_files: false,
                repositories,
                emulator_systems_widget,
                emulator_id: None,
                emulator_systems: vec![],
                is_adding_emulator: false,
                view_model_service,
            },
            task.map(EmulatorAddWidgetMessage::EmulatorSystemsAddWidget),
        )
    }

    pub fn update(&mut self, message: EmulatorAddWidgetMessage) -> Task<EmulatorAddWidgetMessage> {
        println!("EmulatorAddWidget update: {:?}", message);
        match message {
            EmulatorAddWidgetMessage::EmulatorNameChanged(name) => self.emulator_name = name,
            EmulatorAddWidgetMessage::EmulatorExecutableChanged(executable) => {
                self.emulator_executable = executable
            }
            EmulatorAddWidgetMessage::EmulatorExtractFilesChanged(extract_files) => {
                self.emulator_extract_files = extract_files
            }
            EmulatorAddWidgetMessage::Submit => {
                println!("Submitting emulator data...");
                let repositories = Arc::clone(&self.repositories);
                let emulator_name = self.emulator_name.clone();
                let emulator_executable = self.emulator_executable.clone();
                let emulator_extract_files = self.emulator_extract_files;
                let emulator_systems = self
                    .emulator_systems
                    .iter()
                    .map(|es| EmulatorSystemUpdateModel {
                        id: es.id,
                        system_id: es.system_id,
                        arguments: es.arguments.clone(),
                    })
                    .collect::<Vec<_>>();
                if let Some(emulator_id) = self.emulator_id {
                    return Task::perform(
                        async move {
                            repositories
                                .get_emulator_repository()
                                .update_emulator_with_systems(
                                    emulator_id,
                                    emulator_name,
                                    emulator_executable,
                                    emulator_extract_files,
                                    emulator_systems,
                                )
                                .await
                        },
                        EmulatorAddWidgetMessage::EmulatorSaved,
                    );
                } else {
                    return Task::perform(
                        async move {
                            repositories
                                .get_emulator_repository()
                                .add_emulator_with_systems(
                                    emulator_name,
                                    emulator_executable,
                                    emulator_extract_files,
                                    emulator_systems,
                                )
                                .await
                        },
                        EmulatorAddWidgetMessage::EmulatorSaved,
                    );
                }
            }
            EmulatorAddWidgetMessage::EmulatorSaved(result) => match result {
                Ok(id) => {
                    println!("Emulator saved successfully with id: {:?}", id);
                    self.is_adding_emulator = false;
                    let emulator = EmulatorListModel {
                        id,
                        name: self.emulator_name.clone(),
                    };
                    return Task::done(EmulatorAddWidgetMessage::EmulatorAdded(emulator));
                }
                Err(error) => {
                    eprintln!("Error saving emulator: {:?}", error);
                }
            },
            EmulatorAddWidgetMessage::EmulatorSystemsAddWidget(message) => {
                if let emulator_systems_add_widget::EmulatorSystemsAddWidgetMessage::AddEmulatorSystem(emulator_system) =
                    &message
                {
                    if emulator_system.id.is_none() {
                        self.emulator_systems.push(emulator_system.clone());
                    } else if let Some(index) = self
                        .emulator_systems
                        .iter()
                        .position(|es| es.id == emulator_system.id)
                    {
                        self.emulator_systems[index] = emulator_system.clone();
                    }
                }
                return self
                    .emulator_systems_widget
                    .update(message.clone())
                    .map(EmulatorAddWidgetMessage::EmulatorSystemsAddWidget);
            }

            EmulatorAddWidgetMessage::EmulatorSystemSaved(result) => match result {
                Ok(id) => {
                    println!("Emulator system saved successfully with id: {:?}", id);
                    return Task::none();
                }
                Err(error) => {
                    eprintln!("Error saving emulator system: {:?}", error);
                }
            },
            EmulatorAddWidgetMessage::StartAddEmulator => {
                self.is_adding_emulator = true;
            }
            EmulatorAddWidgetMessage::CancelAddEmulator => {
                self.is_adding_emulator = false;
            }
            EmulatorAddWidgetMessage::SetEmulatorId(id) => {
                self.emulator_id = Some(id);
                let viewm_model_service = Arc::clone(&self.view_model_service);
                self.is_adding_emulator = true;
                return Task::perform(
                    async move { viewm_model_service.get_emulator_view_model(id).await },
                    EmulatorAddWidgetMessage::EmulatorFetched,
                );
            }
            EmulatorAddWidgetMessage::EmulatorFetched(result) => match result {
                Ok(emulator) => {
                    println!("Emulator fetched successfully: {:?}", emulator);
                    self.emulator_name = emulator.name;
                    self.emulator_executable = emulator.executable;
                    self.emulator_extract_files = emulator.extract_files;
                    self.emulator_systems = emulator
                        .systems
                        .into_iter()
                        .map(|es| EmulatorSystem {
                            id: Some(es.id),
                            system_id: es.system_id,
                            system_name: es.system_name,
                            arguments: es.arguments,
                        })
                        .collect();
                }
                Err(error) => {
                    eprintln!("Error fetching emulator: {:?}", error);
                }
            },
            EmulatorAddWidgetMessage::EditEmulatorSystem(emulator_system) => return self
                .emulator_systems_widget
                .update(
                    emulator_systems_add_widget::EmulatorSystemsAddWidgetMessage::SetEmulatorSystem(
                        emulator_system,
                    ),
                )
                .map(EmulatorAddWidgetMessage::EmulatorSystemsAddWidget),
            _ => {
                println!("Unhandled message in EmulatorAddWidget: {:?}", message);
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<EmulatorAddWidgetMessage> {
        let emulator_add_view = if self.is_adding_emulator {
            self.create_add_emulator_view_content()
        } else {
            Column::new()
                .push(button("Add Emulator").on_press(EmulatorAddWidgetMessage::StartAddEmulator))
        };
        Container::new(
            emulator_add_view
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING),
        )
        .style(container::bordered_box)
        .into()
    }

    fn create_add_emulator_view_content(&self) -> Column<EmulatorAddWidgetMessage> {
        let cancel_button_text = if self.emulator_id.is_some() {
            "Cancel edit emulator"
        } else {
            "Cancel add emulator"
        };
        let cancel_add_emulator_button =
            button(cancel_button_text).on_press(EmulatorAddWidgetMessage::CancelAddEmulator);

        let name_input = text_input("Emulator name", &self.emulator_name)
            .on_input(EmulatorAddWidgetMessage::EmulatorNameChanged);
        let executable_input =
            iced::widget::text_input("Emulator executable", &self.emulator_executable)
                .on_input(EmulatorAddWidgetMessage::EmulatorExecutableChanged);
        let extract_files_checkbox = checkbox("Extract files", self.emulator_extract_files)
            .on_toggle(EmulatorAddWidgetMessage::EmulatorExtractFilesChanged);

        let emulator_systems_view = self
            .emulator_systems_widget
            .view()
            .map(EmulatorAddWidgetMessage::EmulatorSystemsAddWidget);

        let emulator_systems_list = self
            .emulator_systems
            .iter()
            .map(|system| {
                let label = text!(
                    "System: {} Arguments: {}",
                    system.system_name,
                    system.arguments
                );
                let edit_button = button("Edit")
                    .on_press(EmulatorAddWidgetMessage::EditEmulatorSystem(system.clone()));
                row![label, edit_button]
                    .spacing(DEFAULT_SPACING)
                    .padding(DEFAULT_PADDING)
                    .into()
            })
            .collect::<Vec<Element<EmulatorAddWidgetMessage>>>();

        let submit_button = button("Submit").on_press(EmulatorAddWidgetMessage::Submit);

        let emulator_systems_list = Column::with_children(emulator_systems_list)
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING);
        Column::new()
            .push(cancel_add_emulator_button)
            .push(name_input)
            .push(executable_input)
            .push(extract_files_checkbox)
            .push(emulator_systems_list)
            .push(emulator_systems_view)
            .push(submit_button)
    }
}
