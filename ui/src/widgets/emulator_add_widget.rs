use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use iced::{
    widget::{button, checkbox, container, text, text_input, Column, Container},
    Element, Task,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{EmulatorListModel, EmulatorSystemListModel},
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::emulator_systems_add_widget::{self, EmulatorSystemsAddWidget};

pub struct EmulatorAddWidget {
    emulator_name: String,
    emulator_executable: String,
    emulator_extract_files: bool,
    repositories: Arc<RepositoryManager>,
    emulator_systems_widget: EmulatorSystemsAddWidget,
    emulator_id: Option<i64>,
    emulator_systems: Vec<EmulatorSystemListModel>,
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
            EmulatorSystemsAddWidget::new(Arc::clone(&repositories), view_model_service, None);
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
                return Task::perform(
                    async move {
                        repositories
                            .get_emulator_repository()
                            .add_emulator(
                                emulator_name,
                                emulator_executable,
                                emulator_extract_files,
                            )
                            .await
                    },
                    Message::EmulatorSaved,
                );
            }
            Message::EmulatorSaved(result) => match result {
                Ok(id) => {
                    println!("Emulator saved successfully with id: {:?}", id);
                    let emulator = EmulatorListModel {
                        id,
                        name: self.emulator_name.clone(),
                    };
                    self.emulator_id = Some(id);
                    let set_emulator_id_task = self
                        .emulator_systems_widget
                        .update(emulator_systems_add_widget::Message::SetEmulatorId(id))
                        .map(Message::EmulatorSystemsAddWidget);

                    let emulator_added_task = Task::done(Message::EmulatorAdded(emulator));
                    let combined_task =
                        Task::batch(vec![set_emulator_id_task, emulator_added_task]);
                    return combined_task;
                }
                Err(error) => {
                    eprintln!("Error saving emulator: {:?}", error);
                }
            },
            Message::EmulatorSystemsAddWidget(message) => {
                let update_task = self
                    .emulator_systems_widget
                    .update(message.clone())
                    .map(Message::EmulatorSystemsAddWidget);
                if let emulator_systems_add_widget::Message::AddEmulatorSystem(list_model) =
                    &message
                {
                    self.emulator_systems.push(list_model.clone());
                }
                return update_task;
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
        let add_emulator_button = button("Add Emulator").on_press(Message::StartAddEmulator);
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
            .map(|system| text!("System: {}", system.system_name).into())
            .collect::<Vec<Element<Message>>>();

        let submit_button = button("Submit").on_press(Message::Submit);

        let emulator_systems_list = Column::with_children(emulator_systems_list)
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING);

        let emulator_add_view = if self.is_adding_emulator {
            Column::new()
                .push(cancel_add_emulator_button)
                .push(name_input)
                .push(executable_input)
                .push(extract_files_checkbox)
                .push(emulator_systems_list)
                .push(emulator_systems_view)
                .push(submit_button)
        } else {
            Column::new().push(add_emulator_button)
        };
        Container::new(
            emulator_add_view
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING),
        )
        .style(container::bordered_box)
        .into()
    }
}
