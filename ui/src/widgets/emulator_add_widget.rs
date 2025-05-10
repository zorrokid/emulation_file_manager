use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use iced::{
    widget::{checkbox, text, text_input, Column},
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
            _ => {
                println!("Unhandled message in EmulatorAddWidget: {:?}", message);
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<Message> {
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

        let submit_button = iced::widget::button("Submit").on_press(Message::Submit);

        iced::widget::column![
            name_input,
            executable_input,
            extract_files_checkbox,
            Column::with_children(emulator_systems_list)
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING),
            emulator_systems_view,
            submit_button,
        ]
        .spacing(DEFAULT_SPACING)
        .padding(DEFAULT_PADDING)
        .into()
    }
}
