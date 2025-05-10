use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use iced::{
    widget::{button, column, text, text_input},
    Element, Task,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{EmulatorSystemListModel, SystemListModel},
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::{
    system_select_widget,
    systems_widget::{self, SystemsWidget},
};

pub struct EmulatorSystemsAddWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems_widget: SystemsWidget,
    selected_system: Option<SystemListModel>,
    arguments: String,
    emulator_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Systems(systems_widget::Message),
    ArgumentsChanged(String),
    Submit,
    AddEmulatorSystem(EmulatorSystemListModel),
    EmulatorSystemSaved(Result<i64, Error>),
    SetEmulatorId(i64),
}

impl EmulatorSystemsAddWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
        emulator_id: Option<i64>,
    ) -> (Self, Task<Message>) {
        let (systems_widget, task) =
            SystemsWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        (
            Self {
                repositories,
                view_model_service,
                systems_widget,
                selected_system: None,
                arguments: String::new(),
                emulator_id,
            },
            task.map(Message::Systems),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Systems(message) => {
                if let systems_widget::Message::SystemSelect(
                    system_select_widget::Message::SystemSelected(system),
                ) = &message
                {
                    self.selected_system = Some(system.clone());
                }
                let task = self.systems_widget.update(message);
                task.map(Message::Systems)
            }
            Message::ArgumentsChanged(arguments) => {
                self.arguments = arguments;
                Task::none()
            }
            Message::Submit => {
                if let (Some(system), Some(emulator_id)) = (&self.selected_system, self.emulator_id)
                {
                    let system_id = system.id;
                    self.selected_system = None;
                    self.arguments = String::new();
                    let repositories = Arc::clone(&self.repositories);
                    let arguments = self.arguments.clone();
                    let add_emulator_system_task = Task::perform(
                        async move {
                            repositories
                                .get_emulator_repository()
                                .add_emulator_system(emulator_id, system_id, arguments)
                                .await
                        },
                        Message::EmulatorSystemSaved,
                    );

                    return add_emulator_system_task;
                }
                Task::none()
            }
            Message::EmulatorSystemSaved(result) => match result {
                Ok(id) => {
                    println!("Emulator system saved successfully");
                    let list_model = EmulatorSystemListModel {
                        id,
                        system_name: self.selected_system.as_ref().unwrap().name.clone(),
                    };
                    return Task::done(Message::AddEmulatorSystem(list_model));
                }
                Err(error) => {
                    eprintln!("Error saving emulator system: {}", error);
                    return Task::none();
                }
            },
            Message::SetEmulatorId(id) => {
                self.emulator_id = Some(id);
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let systems_view = self.systems_widget.view().map(Message::Systems);
        let selected_system_name = self.selected_system.as_ref().map_or("None", |s| &s.name);
        let selected_system_text = text!("Selected System: {}", &selected_system_name);
        let add_argument_input = text_input("Add system specific arguments", &self.arguments)
            .on_input(Message::ArgumentsChanged);
        let submit_button = button("Submit").on_press(Message::Submit);
        column![
            selected_system_text,
            add_argument_input,
            systems_view,
            submit_button
        ]
        .spacing(DEFAULT_SPACING)
        .padding(DEFAULT_PADDING)
        .into()
    }
}
