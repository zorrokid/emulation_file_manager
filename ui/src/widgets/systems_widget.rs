use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use iced::{
    widget::{button, column, row, text, Column},
    Element, Task,
};
use service::{error::Error, view_model_service::ViewModelService, view_models::SystemListModel};

use crate::defaults::DEFAULT_SPACING;

use super::{
    system_add_widget::{self, SystemAddWidget, SystemAddWidgetMessage},
    system_select_widget::{self, SystemSelectWidget, SystemSelectWidgetMessage},
};

pub struct SystemsWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems: Vec<SystemListModel>,
    system_select_widget: SystemSelectWidget,
    system_add_widget: SystemAddWidget,
    // TODO: selected systems are also maintained in parent widget!
    selected_system_ids: Vec<i64>,
    adding_system: bool,
}

#[derive(Debug, Clone)]
pub enum SystemWidgetMessage {
    // child messages
    AddSystem(SystemAddWidgetMessage),
    SystemSelect(SystemSelectWidgetMessage),
    // local messages
    SystemsFetched(Result<Vec<SystemListModel>, Error>),
    SystemAdded(Result<i64, DatabaseError>),
    RemoveSystem(i64),
    StartAddSystem,
    CancelAddSystem,
    SetSelectedSystemIds(Vec<i64>),
}

impl SystemsWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<SystemWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_systems_task = Task::perform(
            async move { view_model_service_clone.get_system_list_models().await },
            SystemWidgetMessage::SystemsFetched,
        );

        (
            Self {
                repositories,
                view_model_service,
                systems: vec![],
                system_select_widget: SystemSelectWidget::new(),
                system_add_widget: SystemAddWidget::new(),
                selected_system_ids: vec![],
                adding_system: false,
            },
            fetch_systems_task,
        )
    }

    pub fn update(&mut self, message: SystemWidgetMessage) -> Task<SystemWidgetMessage> {
        match message {
            SystemWidgetMessage::SystemsFetched(result) => match result {
                Ok(systems) => {
                    self.systems = systems;
                    self.system_select_widget
                        .update(system_select_widget::SystemSelectWidgetMessage::SetSystems(
                            self.systems.clone(),
                        ))
                        .map(SystemWidgetMessage::SystemSelect)
                }
                Err(error) => {
                    eprint!("Error when fetching systems: {}", error);
                    Task::none()
                }
            },
            SystemWidgetMessage::AddSystem(message) => {
                if let SystemAddWidgetMessage::AddSystem(name) = message {
                    let repo = Arc::clone(&self.repositories);
                    return Task::perform(
                        async move { repo.get_system_repository().add_system(name).await },
                        SystemWidgetMessage::SystemAdded,
                    );
                }
                Task::none()
            }
            SystemWidgetMessage::SystemSelect(message) => {
                if let SystemSelectWidgetMessage::SystemSelected(system) = message {
                    if !self.selected_system_ids.contains(&system.id) {
                        self.selected_system_ids.push(system.id);
                    }
                }
                Task::none()
            }
            SystemWidgetMessage::SystemAdded(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    Task::perform(
                        async move { service.get_system_list_models().await },
                        SystemWidgetMessage::SystemsFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when adding system: {}", error);
                    Task::none()
                }
            },
            SystemWidgetMessage::RemoveSystem(id) => {
                self.selected_system_ids
                    .retain(|&system_id| system_id != id);
                Task::none()
            }
            SystemWidgetMessage::StartAddSystem => {
                self.adding_system = true;
                Task::none()
            }
            SystemWidgetMessage::CancelAddSystem => {
                self.adding_system = false;
                Task::none()
            }
            SystemWidgetMessage::SetSelectedSystemIds(ids) => {
                self.selected_system_ids = ids;
                // TODO: should this emit SystemSelected message for each system? Then this
                // wouldn't be needed to set explicitly in parent widget which maintains selected
                // systems for the release.
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<SystemWidgetMessage> {
        let add_system_view: Element<SystemWidgetMessage> = if self.adding_system {
            let system_add_view = self
                .system_add_widget
                .view()
                .map(SystemWidgetMessage::AddSystem);
            let cancel_button =
                button("Cancel add system").on_press(SystemWidgetMessage::CancelAddSystem);
            column![cancel_button, system_add_view].into()
        } else {
            button("Add System")
                .on_press(SystemWidgetMessage::StartAddSystem)
                .into()
        };

        let system_select_view: Element<SystemWidgetMessage> = self
            .system_select_widget
            .view()
            .map(SystemWidgetMessage::SystemSelect);
        let selected_systems_list = self.create_selected_systems_list();
        column![system_select_view, selected_systems_list, add_system_view].into()
    }

    fn create_selected_systems_list(&self) -> Element<SystemWidgetMessage> {
        let selected_systems = self
            .selected_system_ids
            .iter()
            .map(|id| {
                let system = self
                    .systems
                    .iter()
                    .find(|system| system.id == *id)
                    .unwrap_or_else(|| panic!("System with id {} not found", id));
                let remove_button =
                    button("Remove").on_press(SystemWidgetMessage::RemoveSystem(*id));
                row![text!("{}", system.name.clone()).width(200.0), remove_button]
                    .spacing(DEFAULT_SPACING)
                    .padding(crate::defaults::DEFAULT_PADDING / 2.0)
                    .into()
            })
            .collect::<Vec<Element<SystemWidgetMessage>>>();

        Column::with_children(selected_systems).into()
    }
}
