use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use iced::{
    widget::{button, column, row, text, Column},
    Element, Task,
};
use service::{error::Error, view_model_service::ViewModelService, view_models::SystemListModel};

use crate::defaults::DEFAULT_SPACING;

use super::{
    system_add_widget::{self, SystemAddWidget},
    system_select_widget::{self, SystemSelectWidget},
};

pub struct SystemsWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems: Vec<SystemListModel>,
    system_select_widget: SystemSelectWidget,
    system_add_widget: SystemAddWidget,
    selected_system_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SystemsFetched(Result<Vec<SystemListModel>, Error>),
    AddSystem(system_add_widget::Message),
    SystemSelect(system_select_widget::Message),
    SystemAdded(Result<i64, DatabaseError>),
    RemoveSystem(i64),
}

impl SystemsWidget {
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
                system_select_widget: SystemSelectWidget::new(),
                system_add_widget: SystemAddWidget::new(),
                selected_system_ids: vec![],
            },
            fetch_systems_task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SystemsFetched(result) => match result {
                Ok(systems) => {
                    self.systems = systems;
                    self.system_select_widget
                        .update(system_select_widget::Message::SetSystems(
                            self.systems.clone(),
                        ))
                        .map(Message::SystemSelect)
                }
                Err(error) => {
                    eprint!("Error when fetching systems: {}", error);
                    Task::none()
                }
            },
            Message::AddSystem(message) => match self.system_add_widget.update(message) {
                system_add_widget::Action::AddSystem(name) => {
                    let repo = Arc::clone(&self.repositories);
                    Task::perform(
                        async move { repo.get_system_repository().add_system(name).await },
                        Message::SystemAdded,
                    )
                }
                system_add_widget::Action::None => Task::none(),
            },
            Message::SystemSelect(message) => {
                if let system_select_widget::Message::SystemSelected(system) = message {
                    self.selected_system_ids.push(system.id);
                    Task::none()
                } else {
                    Task::none()
                }
            }
            Message::SystemAdded(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    Task::perform(
                        async move { service.get_system_list_models().await },
                        Message::SystemsFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when adding system: {}", error);
                    Task::none()
                }
            },
            Message::RemoveSystem(id) => {
                self.selected_system_ids
                    .retain(|&system_id| system_id != id);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let add_system_view = self.system_add_widget.view().map(Message::AddSystem);
        let systems_view = self.system_select_widget.view().map(Message::SystemSelect);
        let selected_systems_list = self.create_selected_systems_list();
        column![add_system_view, systems_view, selected_systems_list].into()
    }

    fn create_selected_systems_list(&self) -> iced::Element<Message> {
        let selected_systems = self
            .selected_system_ids
            .iter()
            .map(|id| {
                let system = self
                    .systems
                    .iter()
                    .find(|system| system.id == *id)
                    .unwrap_or_else(|| panic!("System with id {} not found", id));
                let remove_button = button("Remove").on_press(Message::RemoveSystem(*id));
                row![text!("{}", system.name.clone()).width(200.0), remove_button]
                    .spacing(DEFAULT_SPACING)
                    .padding(crate::defaults::DEFAULT_PADDING / 2.0)
                    .into()
            })
            .collect::<Vec<Element<Message>>>();

        Column::with_children(selected_systems).into()
    }
}
