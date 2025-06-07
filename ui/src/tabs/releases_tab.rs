use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{
    widget::{column, container, scrollable, Container},
    Element, Task,
};
use service::{
    error::Error as ServiceError,
    view_model_service::{ReleaseFilter, ViewModelService},
    view_models::{ReleaseViewModel, SoftwareTitleListModel, SystemListModel},
};

use crate::widgets::{
    release_select_widget::{self, ReleaseSelectWidget, ReleaseSelectWidgetMessage},
    release_view_widget::{ReleaseViewWidget, ReleaseViewWidgetMessage},
    release_widget::{self, ReleaseWidget, ReleaseWidgetMessage},
    software_title_filter_widget::{SoftwareTitleFilterWidget, SoftwareTitleFilterWidgetMessage},
    systems_filter_widget::{SystemFilterWidget, SystemFilterWidgetMessage},
};

pub struct ReleasesTab {
    view_model_service: Arc<ViewModelService>,
    release_select_widget: ReleaseSelectWidget,
    selected_release: Option<ReleaseViewModel>,
    release_edit_widget: ReleaseWidget,
    system_filter: SystemFilterWidget,
    software_titles_filter: SoftwareTitleFilterWidget,
    release_view_widget: ReleaseViewWidget,
    filters: ReleaseFilter,
}

#[derive(Debug, Clone)]
pub enum ReleasesTabMessage {
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
}

impl ReleasesTab {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<ReleasesTabMessage>) {
        let release_select_widget = ReleaseSelectWidget::new(Arc::clone(&view_model_service));

        let (release_widget, release_widget_task) =
            ReleaseWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        let view_model_service_clone = Arc::clone(&view_model_service);
        let load_systems_task = Task::perform(
            async move { view_model_service_clone.get_system_list_models().await },
            ReleasesTabMessage::SystemsFetched,
        );

        let view_model_service_clone = Arc::clone(&view_model_service);
        let load_software_titles_task = Task::perform(
            async move {
                view_model_service_clone
                    .get_software_title_list_models()
                    .await
            },
            ReleasesTabMessage::SoftwareTitlesFetched,
        );

        let view_model_service_clone = Arc::clone(&view_model_service);
        let (release_view_widget, release_view_task) =
            ReleaseViewWidget::new(view_model_service_clone);

        let combined_task = Task::batch(vec![
            release_widget_task.map(ReleasesTabMessage::ReleaseWidget),
            load_systems_task,
            load_software_titles_task,
            release_view_task.map(ReleasesTabMessage::ReleaseViewWidget),
        ]);

        (
            Self {
                view_model_service,
                release_select_widget,
                selected_release: None,
                release_edit_widget: release_widget,
                system_filter: SystemFilterWidget::new(),
                software_titles_filter: SoftwareTitleFilterWidget::new(),
                release_view_widget,
                filters: ReleaseFilter {
                    system_id: None,
                    software_title_id: None,
                },
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: ReleasesTabMessage) -> Task<ReleasesTabMessage> {
        match message {
            ReleasesTabMessage::ReleaseSelectWidget(message) => {
                let update_task = self
                    .release_select_widget
                    .update(message.clone())
                    .map(ReleasesTabMessage::ReleaseSelectWidget);

                if let release_select_widget::ReleaseSelectWidgetMessage::SetReleaseSelected(
                    release_id,
                ) = message.clone()
                {
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let fetch_selected_release_task = Task::perform(
                        async move { view_model_service.get_release_view_model(release_id).await },
                        ReleasesTabMessage::ReleaseFetched,
                    );
                    let combined_task = Task::batch(vec![update_task, fetch_selected_release_task]);
                    return combined_task;
                }
                if let release_select_widget::ReleaseSelectWidgetMessage::ClearSelectedRelease =
                    message
                {
                    self.selected_release = None;

                    let clear_release_task = self
                        .release_edit_widget
                        .update(ReleaseWidgetMessage::ClearRelease)
                        .map(ReleasesTabMessage::ReleaseWidget);

                    let clear_view_release_task = self
                        .release_view_widget
                        .update(ReleaseViewWidgetMessage::ClearRelease)
                        .map(ReleasesTabMessage::ReleaseViewWidget);

                    return Task::batch(vec![clear_release_task, clear_view_release_task]);
                }

                update_task
            }
            ReleasesTabMessage::ReleaseFetched(result) => match result {
                Ok(release_view_model) => {
                    self.selected_release = Some(release_view_model.clone());
                    self.release_view_widget
                        .update(ReleaseViewWidgetMessage::SetRelease(release_view_model))
                        .map(ReleasesTabMessage::ReleaseViewWidget)
                }
                Err(err) => {
                    eprintln!("Error fetching release: {}", err);
                    Task::none()
                }
            },
            ReleasesTabMessage::ReleaseWidget(message) => self
                .release_edit_widget
                .update(message.clone())
                .map(ReleasesTabMessage::ReleaseWidget),
            ReleasesTabMessage::SystemsFetched(result) => match result {
                Ok(systems) => {
                    let task: Task<ReleasesTabMessage> = self
                        .system_filter
                        .update(SystemFilterWidgetMessage::SetSystems(systems))
                        .map(ReleasesTabMessage::SystemFilterWidget);
                    task
                }
                Err(err) => {
                    eprintln!("Error fetching systems: {}", err);
                    Task::none()
                }
            },
            ReleasesTabMessage::SoftwareTitlesFetched(result) => match result {
                Ok(software_titles) => {
                    let task: Task<ReleasesTabMessage> = self
                        .software_titles_filter
                        .update(SoftwareTitleFilterWidgetMessage::SetSoftwareTitles(
                            software_titles,
                        ))
                        .map(ReleasesTabMessage::SoftwareTitleFilterWidget);
                    task
                }
                Err(err) => {
                    eprintln!("Error fetching software titles: {}", err);
                    Task::none()
                }
            },
            ReleasesTabMessage::SoftwareTitleFilterWidget(message) => {
                let update_task = self
                    .software_titles_filter
                    .update(message.clone())
                    .map(ReleasesTabMessage::SoftwareTitleFilterWidget);
                if let SoftwareTitleFilterWidgetMessage::SetSelectedSoftwareTitle(
                    software_title_id,
                ) = message
                {
                    self.filters.software_title_id = software_title_id;
                    return self
                        .release_select_widget
                        .update(
                            release_select_widget::ReleaseSelectWidgetMessage::SetFilters(
                                self.filters.clone(),
                            ),
                        )
                        .map(ReleasesTabMessage::ReleaseSelectWidget);
                }
                update_task
            }
            ReleasesTabMessage::SystemFilterWidget(message) => {
                let update_task = self
                    .system_filter
                    .update(message.clone())
                    .map(ReleasesTabMessage::SystemFilterWidget);
                if let SystemFilterWidgetMessage::SetSelectedSystem(system_id) = message {
                    self.filters.system_id = system_id;
                    return self
                        .release_select_widget
                        .update(
                            release_select_widget::ReleaseSelectWidgetMessage::SetFilters(
                                self.filters.clone(),
                            ),
                        )
                        .map(ReleasesTabMessage::ReleaseSelectWidget);
                }
                update_task
            }
            ReleasesTabMessage::ReleaseViewWidget(message) => {
                let mut tasks: Vec<Task<ReleasesTabMessage>> = vec![self
                    .release_view_widget
                    .update(message.clone())
                    .map(ReleasesTabMessage::ReleaseViewWidget)];
                if let ReleaseViewWidgetMessage::SetEditRelease(release) = &message {
                    tasks.push(
                        self.release_edit_widget
                            .update(release_widget::ReleaseWidgetMessage::SetSelectedRelease(
                                release.clone(),
                            ))
                            .map(ReleasesTabMessage::ReleaseWidget),
                    );
                }
                Task::batch(tasks)
            }
        }
    }

    pub fn view(&self) -> Element<ReleasesTabMessage> {
        column![
            self.create_filter_pane(),
            self.release_view_widget
                .view()
                .map(ReleasesTabMessage::ReleaseViewWidget),
            scrollable(
                self.release_edit_widget
                    .view()
                    .map(ReleasesTabMessage::ReleaseWidget),
            )
        ]
        .into()
    }

    fn create_filter_pane(&self) -> Container<ReleasesTabMessage> {
        let system_filter_view = self
            .system_filter
            .view()
            .map(ReleasesTabMessage::SystemFilterWidget);
        let software_title_filter_view = self
            .software_titles_filter
            .view()
            .map(ReleasesTabMessage::SoftwareTitleFilterWidget);
        let release_select_view = self
            .release_select_widget
            .view()
            .map(ReleasesTabMessage::ReleaseSelectWidget);

        Container::new(column![
            system_filter_view,
            software_title_filter_view,
            release_select_view,
        ])
        .style(container::bordered_box)
    }
}
