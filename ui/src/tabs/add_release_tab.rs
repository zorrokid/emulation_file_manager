use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{
    widget::{column, container, Container},
    Task,
};
use service::{
    error::Error as ServiceError,
    view_model_service::ViewModelService,
    view_models::{ReleaseViewModel, SoftwareTitleListModel, SystemListModel},
};

use crate::widgets::{
    release_select_widget::{self, ReleaseSelectWidget, ReleaseSelectWidgetMessage},
    release_view_widget::{ReleaseViewWidget, ReleaseViewWidgetMessage},
    release_widget::{self, ReleaseWidget, ReleaseWidgetMessage},
    software_title_filter_widget::{SoftwareTitleFilterWidget, SoftwareTitleFilterWidgetMessage},
    systems_filter_widget::{SystemFilterWidget, SystemFilterWidgetMessage},
};

pub struct AddReleaseTab {
    view_model_service: Arc<ViewModelService>,
    release_select_widget: ReleaseSelectWidget,
    selected_release: Option<ReleaseViewModel>,
    release_widget: ReleaseWidget,
    system_filter: SystemFilterWidget,
    software_titles_filter: SoftwareTitleFilterWidget,
    selected_software_title: Option<SoftwareTitleListModel>,
    selected_system: Option<SystemListModel>,
    release_view_widget: ReleaseViewWidget,
}

#[derive(Debug, Clone)]
pub enum AddReleaseTabMessage {
    // child messages
    ReleaseSelectWidget(ReleaseSelectWidgetMessage),
    ReleaseWidget(ReleaseWidgetMessage),
    SystemFilterWidget(SystemFilterWidgetMessage),
    SoftwareTitleFilterWidget(SoftwareTitleFilterWidgetMessage),
    ReleaseViewWidget(ReleaseViewWidgetMessage),
    // local messages
    ReleaseFetched(Result<ReleaseViewModel, ServiceError>),
    SystemsFetched(Result<Vec<SystemListModel>, ServiceError>),
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, ServiceError>),
    SystemChanged(SystemListModel),
}

impl AddReleaseTab {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<AddReleaseTabMessage>) {
        let (release_select_widget, release_select_task) =
            ReleaseSelectWidget::new(Arc::clone(&view_model_service));

        let (release_widget, release_widget_task) =
            ReleaseWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        let view_model_service_clone = Arc::clone(&view_model_service);
        let load_systems_task = Task::perform(
            async move { view_model_service_clone.get_system_list_models().await },
            AddReleaseTabMessage::SystemsFetched,
        );

        let view_model_service_clone = Arc::clone(&view_model_service);
        let load_software_titles_task = Task::perform(
            async move {
                view_model_service_clone
                    .get_software_title_list_models()
                    .await
            },
            AddReleaseTabMessage::SoftwareTitlesFetched,
        );

        let view_model_service_clone = Arc::clone(&view_model_service);
        let (release_view_widget, release_view_task) =
            ReleaseViewWidget::new(view_model_service_clone);

        let combined_task = Task::batch(vec![
            release_select_task.map(AddReleaseTabMessage::ReleaseSelectWidget),
            release_widget_task.map(AddReleaseTabMessage::ReleaseWidget),
            load_systems_task,
            load_software_titles_task,
            release_view_task.map(AddReleaseTabMessage::ReleaseViewWidget),
        ]);

        (
            Self {
                view_model_service,
                release_select_widget,
                selected_release: None,
                release_widget,
                system_filter: SystemFilterWidget::new(),
                software_titles_filter: SoftwareTitleFilterWidget::new(),
                selected_software_title: None,
                selected_system: None,
                release_view_widget,
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: AddReleaseTabMessage) -> Task<AddReleaseTabMessage> {
        match message {
            AddReleaseTabMessage::ReleaseSelectWidget(message) => {
                let update_task = self
                    .release_select_widget
                    .update(message.clone())
                    .map(AddReleaseTabMessage::ReleaseSelectWidget);

                if let release_select_widget::ReleaseSelectWidgetMessage::SetReleaseSelected(
                    release_id,
                ) = message.clone()
                {
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let fetch_selected_release_task = Task::perform(
                        async move { view_model_service.get_release_view_model(release_id).await },
                        AddReleaseTabMessage::ReleaseFetched,
                    );
                    let combined_task = Task::batch(vec![update_task, fetch_selected_release_task]);
                    return combined_task;
                }
                if let release_select_widget::ReleaseSelectWidgetMessage::ClearSelectedRelease =
                    message
                {
                    self.selected_release = None;

                    let clear_release_task = self
                        .release_widget
                        .update(ReleaseWidgetMessage::ClearRelease)
                        .map(AddReleaseTabMessage::ReleaseWidget);

                    let clear_view_release_task = self
                        .release_view_widget
                        .update(ReleaseViewWidgetMessage::ClearRelease)
                        .map(AddReleaseTabMessage::ReleaseViewWidget);

                    return Task::batch(vec![clear_release_task, clear_view_release_task]);
                }

                update_task
            }
            AddReleaseTabMessage::ReleaseFetched(result) => match result {
                Ok(release_view_model) => {
                    println!("Got ReleaseFetched message.");
                    self.selected_release = Some(release_view_model.clone());
                    self.release_view_widget
                        .update(ReleaseViewWidgetMessage::SetRelease(release_view_model))
                        .map(AddReleaseTabMessage::ReleaseViewWidget)
                }
                Err(err) => {
                    eprintln!("Error fetching release: {}", err);
                    Task::none()
                }
            },
            AddReleaseTabMessage::ReleaseWidget(message) => {
                let update_task = self
                    .release_widget
                    .update(message.clone())
                    .map(AddReleaseTabMessage::ReleaseWidget);

                if let release_widget::ReleaseWidgetMessage::ReleaseWasUpdated(release_list_model) =
                    message
                {
                    // TODO: update selected release
                }

                update_task
            }
            AddReleaseTabMessage::SystemChanged(system) => {
                println!("System changed: {:?}", system);
                Task::none()
            }
            AddReleaseTabMessage::SystemsFetched(result) => match result {
                Ok(systems) => {
                    println!("Got systems: {:?}", systems);
                    let task: Task<AddReleaseTabMessage> = self
                        .system_filter
                        .update(SystemFilterWidgetMessage::SetSystems(systems))
                        .map(AddReleaseTabMessage::SystemFilterWidget);
                    task
                }
                Err(err) => {
                    eprintln!("Error fetching systems: {}", err);
                    Task::none()
                }
            },
            AddReleaseTabMessage::SoftwareTitlesFetched(result) => match result {
                Ok(software_titles) => {
                    println!("Got software titles: {:?}", software_titles);
                    let task: Task<AddReleaseTabMessage> = self
                        .software_titles_filter
                        .update(SoftwareTitleFilterWidgetMessage::SetSoftwareTitles(
                            software_titles,
                        ))
                        .map(AddReleaseTabMessage::SoftwareTitleFilterWidget);
                    task
                }
                Err(err) => {
                    eprintln!("Error fetching software titles: {}", err);
                    Task::none()
                }
            },
            AddReleaseTabMessage::SoftwareTitleFilterWidget(message) => {
                let update_task = self
                    .software_titles_filter
                    .update(message.clone())
                    .map(AddReleaseTabMessage::SoftwareTitleFilterWidget);
                if let SoftwareTitleFilterWidgetMessage::SoftwareTitleSelected(software_title) =
                    message
                {
                    println!("Software title selected: {:?}", software_title);
                    self.selected_software_title = Some(software_title.clone());
                }
                update_task
            }
            AddReleaseTabMessage::SystemFilterWidget(message) => {
                let update_task = self
                    .system_filter
                    .update(message.clone())
                    .map(AddReleaseTabMessage::SystemFilterWidget);
                if let SystemFilterWidgetMessage::SetSelectedSystem(system) = message {
                    println!("System selected: {:?}", system);
                    self.selected_system = Some(system.clone());
                }
                update_task
            }
            AddReleaseTabMessage::ReleaseViewWidget(message) => {
                if let ReleaseViewWidgetMessage::SetEditRelease(release) = message {
                    return self
                        .release_widget
                        .update(release_widget::ReleaseWidgetMessage::SetSelectedRelease(
                            release,
                        ))
                        .map(AddReleaseTabMessage::ReleaseWidget);
                }
                self.release_view_widget
                    .update(message.clone())
                    .map(AddReleaseTabMessage::ReleaseViewWidget)
            }
            _ => {
                println!("Unhandled message: {:?}", message);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<AddReleaseTabMessage> {
        let release_view = self
            .release_widget
            .view()
            .map(AddReleaseTabMessage::ReleaseWidget);
        let selected_release_view = self
            .release_view_widget
            .view()
            .map(AddReleaseTabMessage::ReleaseViewWidget);
        column![
            self.create_filter_pane(),
            selected_release_view,
            release_view
        ]
        .into()
    }

    fn create_filter_pane(&self) -> Container<AddReleaseTabMessage> {
        let system_filter_view = self
            .system_filter
            .view()
            .map(AddReleaseTabMessage::SystemFilterWidget);
        let software_title_filter_view = self
            .software_titles_filter
            .view()
            .map(AddReleaseTabMessage::SoftwareTitleFilterWidget);
        let release_select_view = self
            .release_select_widget
            .view()
            .map(AddReleaseTabMessage::ReleaseSelectWidget);

        Container::new(column![
            system_filter_view,
            software_title_filter_view,
            release_select_view,
        ])
        .style(container::bordered_box)
    }
}
