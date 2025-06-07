use std::sync::Arc;

use database::{
    database_error::Error as DatabaseError, models::System, repository_manager::RepositoryManager,
};
use iced::{
    widget::{button, column, container, row, text, Column, Container},
    Element, Length, Task,
};
use service::{error::Error, view_model_service::ViewModelService, view_models::SystemListModel};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::{
    system_add_widget::{SystemAddWidget, SystemAddWidgetMessage},
    system_select_widget::{self, SystemSelectWidget, SystemSelectWidgetMessage},
};

pub struct SystemsWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems: Vec<SystemListModel>,
    system_select_widget: SystemSelectWidget,
    system_add_widget: SystemAddWidget,
    is_edit_mode: bool,
}

#[derive(Debug, Clone)]
pub enum SystemWidgetMessage {
    Reset,
    StartEditMode(Option<i64>),
    // child messages
    SystemAddWidget(SystemAddWidgetMessage),
    SystemSelectWidget(SystemSelectWidgetMessage),
    // local messages
    SystemsFetched(Result<Vec<SystemListModel>, Error>, Option<i64>),
    SystemAdded(Result<System, DatabaseError>),
    SystemUpdated(Result<System, DatabaseError>),
    RemoveSystem(i64),
    StartEditSystem(i64),
    SetSelectedSystemIds(Vec<i64>),
    StartAddSystem,
    CancelAddSystem,
}

impl SystemsWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<SystemWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_systems_task = Task::perform(
            async move { view_model_service_clone.get_system_list_models().await },
            |result| SystemWidgetMessage::SystemsFetched(result, None),
        );

        (
            Self {
                repositories,
                view_model_service,
                systems: vec![],
                system_select_widget: SystemSelectWidget::new(),
                system_add_widget: SystemAddWidget::new(),
                is_edit_mode: false,
            },
            fetch_systems_task,
        )
    }

    pub fn update(&mut self, message: SystemWidgetMessage) -> Task<SystemWidgetMessage> {
        match message {
            SystemWidgetMessage::SystemsFetched(result, optional_id) => match result {
                Ok(systems) => {
                    self.systems = systems;
                    Task::batch([if let Some(id) = optional_id {
                        Task::done(SystemWidgetMessage::StartEditSystem(id))
                    } else {
                        Task::none()
                    }])
                }
                Err(error) => {
                    eprint!("Error when fetching systems: {}", error);
                    Task::none()
                }
            },
            SystemWidgetMessage::SystemAddWidget(message) => match message {
                SystemAddWidgetMessage::AddSystem(name) => {
                    let repo = Arc::clone(&self.repositories);
                    Task::perform(
                        async move {
                            let result = repo.get_system_repository().add_system(&name).await;
                            match result {
                                Ok(system_id) => {
                                    let system = System {
                                        id: system_id,
                                        name,
                                    };
                                    Ok(system)
                                }
                                Err(e) => Err(e),
                            }
                        },
                        SystemWidgetMessage::SystemAdded,
                    )
                }
                SystemAddWidgetMessage::UpdateSystem(id, name) => {
                    let repo = Arc::clone(&self.repositories);
                    Task::perform(
                        async move {
                            let result =
                                repo.get_system_repository().update_system(id, &name).await;
                            match result {
                                Ok(id) => {
                                    let system = System { id, name };
                                    Ok(system)
                                }
                                Err(e) => Err(e),
                            }
                        },
                        SystemWidgetMessage::SystemAdded,
                    )
                }
                _ => self
                    .system_add_widget
                    .update(message)
                    .map(SystemWidgetMessage::SystemAddWidget),
            },
            SystemWidgetMessage::SystemAdded(result) => {
                match result {
                    Ok(system) => {
                        self.systems.push(SystemListModel {
                            id: system.id,
                            name: system.name.clone(),
                            can_delete: true, // newly added systems can be deleted
                        });
                    }
                    Err(error) => {
                        eprint!("Error when adding system: {}", error);
                    }
                }
                Task::none()
            }
            SystemWidgetMessage::SystemUpdated(result) => {
                match result {
                    Ok(system) => {
                        if let Some(existing_system) =
                            self.systems.iter_mut().find(|s| s.id == system.id)
                        {
                            existing_system.name = system.name.clone();
                        } else {
                            eprintln!("System with id {} not found for update", system.id);
                        }
                    }
                    Err(error) => {
                        eprint!("Error when updating system: {}", error);
                    }
                }
                Task::none()
            }
            SystemWidgetMessage::StartAddSystem => {
                self.is_edit_mode = true;
                Task::none()
            }
            SystemWidgetMessage::CancelAddSystem => {
                self.is_edit_mode = false;
                self.system_add_widget
                    .update(SystemAddWidgetMessage::Reset)
                    .map(SystemWidgetMessage::SystemAddWidget)
            }
            SystemWidgetMessage::StartEditSystem(id) => {
                if let Some(system) = self.systems.iter().find(|s| s.id == id) {
                    self.is_edit_mode = true;
                    self.system_add_widget
                        .update(SystemAddWidgetMessage::SetEditSystem(
                            system.id,
                            system.name.clone(),
                        ))
                        .map(SystemWidgetMessage::SystemAddWidget)
                } else {
                    eprintln!("System with id {} not found for editing", id);
                    Task::none()
                }
            }
            SystemWidgetMessage::Reset => self
                .system_select_widget
                .update(system_select_widget::SystemSelectWidgetMessage::Reset)
                .map(SystemWidgetMessage::SystemSelectWidget),
            SystemWidgetMessage::StartEditMode(optional_id) => {
                let view_model_service_clone = Arc::clone(&self.view_model_service);
                Task::perform(
                    async move { view_model_service_clone.get_system_list_models().await },
                    move |result| SystemWidgetMessage::SystemsFetched(result, optional_id),
                )
            }
            SystemWidgetMessage::SystemSelectWidget(message) => self
                .system_select_widget
                .update(message)
                .map(SystemWidgetMessage::SystemSelectWidget),
            _ => Task::none(),
        }
    }

    pub fn view(&self, selected_system_ids: &[i64]) -> Element<SystemWidgetMessage> {
        let add_system_view: Element<SystemWidgetMessage> = if self.is_edit_mode {
            let system_add_view = self
                .system_add_widget
                .view()
                .map(SystemWidgetMessage::SystemAddWidget);
            let cancel_button = button("Cancel").on_press(SystemWidgetMessage::CancelAddSystem);
            column![cancel_button, system_add_view].into()
        } else {
            button("Add System")
                .on_press(SystemWidgetMessage::StartAddSystem)
                .into()
        };

        let system_select_view: Element<SystemWidgetMessage> = self
            .system_select_widget
            .view(&self.systems)
            .map(SystemWidgetMessage::SystemSelectWidget);
        let selected_systems_list = self.create_selected_systems_list(selected_system_ids);
        let content = column![system_select_view, selected_systems_list, add_system_view];
        Container::new(content)
            .style(container::bordered_box)
            .padding(DEFAULT_PADDING)
            .width(Length::Fill)
            .into()
    }

    fn create_selected_systems_list(
        &self,
        selected_system_ids: &[i64],
    ) -> Element<SystemWidgetMessage> {
        let selected_systems = selected_system_ids
            .iter()
            .map(|id| {
                let system = self
                    .systems
                    .iter()
                    .find(|system| system.id == *id)
                    .unwrap_or_else(|| panic!("System with id {} not found", id));
                let remove_button =
                    button("Remove").on_press(SystemWidgetMessage::RemoveSystem(*id));
                let edit_button =
                    button("Edit").on_press(SystemWidgetMessage::StartEditSystem(*id));
                row![
                    text!("{}", system.name.clone()).width(200.0),
                    edit_button,
                    remove_button
                ]
                .spacing(DEFAULT_SPACING)
                .padding(crate::defaults::DEFAULT_PADDING / 2.0)
                .into()
            })
            .collect::<Vec<Element<SystemWidgetMessage>>>();

        Column::with_children(selected_systems).into()
    }
}
