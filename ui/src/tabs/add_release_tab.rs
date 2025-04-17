use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{widget::text, Task};
use service::{error::Error, view_model_service::ViewModelService, view_models::SystemListModel};

use crate::widgets::{
    add_system_widget::{self, AddSystemWidget},
    systems_widget::SystemsWidget,
};

pub struct AddReleaseTab {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems: Vec<SystemListModel>,
    systems_widget: SystemsWidget,
    add_system_widget: AddSystemWidget,
}

#[derive(Debug, Clone)]
pub enum Message {
    SystemsFetched(Result<Vec<SystemListModel>, Error>),
    AddSystem(add_system_widget::Message),
}

impl AddReleaseTab {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_systems_task = Task::perform(
            async move { view_model_service_clone.get_system_list_models().await },
            Message::SystemsFetched,
        );

        (
            Self {
                repositories,
                view_model_service,
                systems: vec![],
                systems_widget: SystemsWidget::new(),
                add_system_widget: AddSystemWidget::new(),
            },
            fetch_systems_task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SystemsFetched(result) => {
                match result {
                    Ok(systems) => {
                        self.systems = systems;
                    }
                    Err(error) => {
                        eprint!("Error when fetching systems: {}", error);
                    }
                }
                Task::none()
            }
            Message::AddSystem(message) => self
                .add_system_widget
                .update(message)
                .map(Message::AddSystem),
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let add_system_view = self.add_system_widget.view().map(Message::AddSystem);
        add_system_view
    }
}
