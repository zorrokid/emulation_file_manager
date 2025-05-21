use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{
    widget::{button, column, text},
    Task,
};
use service::{
    error::Error as ServiceError, view_model_service::ViewModelService,
    view_models::ReleaseViewModel,
};

use crate::widgets::{
    release_select_widget::{self, ReleaseSelectWidget, ReleaseSelectWidgetMessage},
    release_widget::{self, ReleaseWidget, ReleaseWidgetMessage},
};

pub struct AddReleaseTab {
    view_model_service: Arc<ViewModelService>,
    release_select_widget: ReleaseSelectWidget,
    selected_release: Option<ReleaseViewModel>,
    release_widget: ReleaseWidget,
}

#[derive(Debug, Clone)]
pub enum AddReleaseTabMessage {
    // child messages
    ReleaseSelectWidget(ReleaseSelectWidgetMessage),
    ReleaseWidget(ReleaseWidgetMessage),
    // local messages
    ReleaseFetched(Result<ReleaseViewModel, ServiceError>),
    StartEditRelease,
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
        let combined_task = Task::batch(vec![
            release_select_task.map(AddReleaseTabMessage::ReleaseSelectWidget),
            release_widget_task.map(AddReleaseTabMessage::ReleaseWidget),
        ]);

        (
            Self {
                view_model_service,
                release_select_widget,
                selected_release: None,
                release_widget,
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: AddReleaseTabMessage) -> Task<AddReleaseTabMessage> {
        match message {
            AddReleaseTabMessage::ReleaseSelectWidget(message) => {
                println!("ReleaseSelect message");
                let update_task = self
                    .release_select_widget
                    .update(message.clone())
                    .map(AddReleaseTabMessage::ReleaseSelectWidget);

                if let release_select_widget::ReleaseSelectWidgetMessage::SetReleaseSelected(
                    release_id,
                ) = message.clone()
                {
                    println!("Got ReleaseSelected message");
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let fetch_selected_release_task = Task::perform(
                        async move { view_model_service.get_release_view_model(release_id).await },
                        AddReleaseTabMessage::ReleaseFetched,
                    );
                    let combined_task = Task::batch(vec![update_task, fetch_selected_release_task]);
                    return combined_task;
                }

                update_task
            }
            AddReleaseTabMessage::ReleaseFetched(result) => match result {
                Ok(release_view_model) => {
                    println!("Got ReleaseFetched message.");
                    self.selected_release = Some(release_view_model.clone());
                    Task::none()
                }
                Err(err) => {
                    eprintln!("Error fetching release: {}", err);
                    Task::none()
                }
            },
            AddReleaseTabMessage::StartEditRelease => {
                if let Some(release) = self.selected_release.clone() {
                    println!("Editing release: {:?}", release);
                    return self
                        .release_widget
                        .update(release_widget::ReleaseWidgetMessage::SetSelectedRelease(
                            release,
                        ))
                        .map(AddReleaseTabMessage::ReleaseWidget);
                }
                Task::none()
            }
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
        }
    }

    pub fn view(&self) -> iced::Element<AddReleaseTabMessage> {
        let release_select_view = self
            .release_select_widget
            .view()
            .map(AddReleaseTabMessage::ReleaseSelectWidget);
        let selected_release_view = self.create_selected_release_view();
        let release_view = self
            .release_widget
            .view()
            .map(AddReleaseTabMessage::ReleaseWidget);
        column![release_select_view, selected_release_view, release_view].into()
    }

    fn create_selected_release_view(&self) -> iced::Element<AddReleaseTabMessage> {
        if let Some(release) = &self.selected_release {
            let release_name_field = text!("Release Name: {}", release.name);
            let software_titles_field = text!("Software Titles: {:?}", release.software_titles);
            let system_names_field = text!("Systems: {:?}", release.systems);
            let edit_button = button("Edit").on_press(AddReleaseTabMessage::StartEditRelease);

            return column![
                release_name_field,
                software_titles_field,
                system_names_field,
                edit_button,
            ]
            .into();
        }
        iced::widget::text("No release selected").into()
    }
}
