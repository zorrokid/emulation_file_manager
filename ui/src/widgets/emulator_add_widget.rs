use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use iced::{
    widget::{button, checkbox, container, text, text_input, Column, Container},
    Element, Task,
};
use service::{view_model_service::ViewModelService, view_models::EmulatorListModel};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::emulator_systems_add_widget::{self, EmulatorSystemsAddWidget};

#[derive(Debug, Clone)]
pub struct EmulatorSystem {
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
}

#[derive(Debug, Clone)]
pub enum Message {
    EmulatorNameChanged(String),
    EmulatorExecutableChanged(String),
    EmulatorExtractFilesChanged(bool),
    Submit,
    EmulatorSaved(Result<i64, Error>),
    EmulatorAdded(EmulatorListModel),
    EmulatorSystemsAddWidget(emulator_systems_add_widget::Message),
    EmulatorSystemSaved(Result<i64, Error>),
    StartAddEmulator,
    CancelAddEmulator,
}

impl EmulatorAddWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let (emulator_systems_widget, task) =
            EmulatorSystemsAddWidget::new(Arc::clone(&repositories), view_model_service);
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
            },
            task.map(Message::EmulatorSystemsAddWidget),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        println!("EmulatorAddWidget update: {:?}", message);
        match message {
            Message::EmulatorNameChanged(name) => self.emulator_name = name,
            Message::EmulatorExecutableChanged(executable) => self.emulator_executable = executable,
            Message::EmulatorExtractFilesChanged(extract_files) => {
                self.emulator_extract_files = extract_files
            }
            Message::Submit => {
                println!("Submitting emulator data...");
                let repositories = Arc::clone(&self.repositories);
                let emulator_name = self.emulator_name.clone();
                let emulator_executable = self.emulator_executable.clone();
                let emulator_extract_files = self.emulator_extract_files;
                let emulator_systems = self
                    .emulator_systems
                    .iter()
                    .map(|es| (es.system_id, es.arguments.clone()))
                    .collect::<Vec<_>>();
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
                    Message::EmulatorSaved,
                );
            }
            Message::EmulatorSaved(result) => match result {
                Ok(id) => {
                    println!("Emulator saved successfully with id: {:?}", id);
                    self.is_adding_emulator = false;
                    let emulator = EmulatorListModel {
                        id,
                        name: self.emulator_name.clone(),
                    };
                    return Task::done(Message::EmulatorAdded(emulator));
                }
                Err(error) => {
                    eprintln!("Error saving emulator: {:?}", error);
                }
            },
            Message::EmulatorSystemsAddWidget(message) => {
                if let emulator_systems_add_widget::Message::AddEmulatorSystem(emulator_system) =
                    &message
                {
                    self.emulator_systems.push(emulator_system.clone());
                }
                return self
                    .emulator_systems_widget
                    .update(message.clone())
                    .map(Message::EmulatorSystemsAddWidget);
            }

            Message::EmulatorSystemSaved(result) => match result {
                Ok(id) => {
                    println!("Emulator system saved successfully with id: {:?}", id);
                    return Task::none();
                }
                Err(error) => {
                    eprintln!("Error saving emulator system: {:?}", error);
                }
            },
            Message::StartAddEmulator => {
                self.is_adding_emulator = true;
            }
            Message::CancelAddEmulator => {
                self.is_adding_emulator = false;
            }
            _ => {
                println!("Unhandled message in EmulatorAddWidget: {:?}", message);
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
        let emulator_add_view = if self.is_adding_emulator {
            self.create_add_emulator_view_content()
        } else {
            Column::new().push(button("Add Emulator").on_press(Message::StartAddEmulator))
        };
        Container::new(
            emulator_add_view
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING),
        )
        .style(container::bordered_box)
        .into()
    }

    fn create_add_emulator_view_content(&self) -> Column<Message> {
        let cancel_add_emulator_button =
            button("Cancel add emulator").on_press(Message::CancelAddEmulator);

        let name_input =
            text_input("Emulator name", &self.emulator_name).on_input(Message::EmulatorNameChanged);
        let executable_input =
            iced::widget::text_input("Emulator executable", &self.emulator_executable)
                .on_input(Message::EmulatorExecutableChanged);
        let extract_files_checkbox = checkbox("Extract files", self.emulator_extract_files)
            .on_toggle(Message::EmulatorExtractFilesChanged);

        let emulator_systems_view = self
            .emulator_systems_widget
            .view()
            .map(Message::EmulatorSystemsAddWidget);

        let emulator_systems_list = self
            .emulator_systems
            .iter()
            .map(|system| {
                text!(
                    "System: {} Arguments: {}",
                    system.system_name,
                    system.arguments
                )
                .into()
            })
            .collect::<Vec<Element<Message>>>();

        let submit_button = button("Submit").on_press(Message::Submit);

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
