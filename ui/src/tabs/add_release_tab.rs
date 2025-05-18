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
    release_select_widget::{self, ReleaseSelectWidget},
    release_widget::{self, ReleaseWidget},
};

pub struct AddReleaseTab {
    view_model_service: Arc<ViewModelService>,
    release_select_widget: ReleaseSelectWidget,
    selected_release: Option<ReleaseViewModel>,
    release_widget: ReleaseWidget,
}

#[derive(Debug, Clone)]
pub enum Message {
    ReleaseSelect(release_select_widget::Message),
    ReleaseFetched(Result<ReleaseViewModel, ServiceError>),
    StartEditRelease,
    ReleaseWidget(release_widget::Message),
}

impl AddReleaseTab {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let (release_select_widget, release_select_task) =
            ReleaseSelectWidget::new(Arc::clone(&view_model_service));

        let (release_widget, release_widget_task) =
            ReleaseWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));
        let combined_task = Task::batch(vec![
            release_select_task.map(Message::ReleaseSelect),
            release_widget_task.map(Message::ReleaseWidget),
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ReleaseSelect(message) => {
                println!("ReleaseSelect message");
                let update_task = self
                    .release_select_widget
                    .update(message.clone())
                    .map(Message::ReleaseSelect);

                if let release_select_widget::Message::SetReleaseSelected(release_id) =
                    message.clone()
                {
                    println!("Got ReleaseSelected message");
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let fetch_selected_release_task = Task::perform(
                        async move { view_model_service.get_release_view_model(release_id).await },
                        Message::ReleaseFetched,
                    );
                    let combined_task = Task::batch(vec![update_task, fetch_selected_release_task]);
                    return combined_task;
                }

                update_task
            }
            Message::ReleaseFetched(result) => match result {
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
            Message::StartEditRelease => {
                if let Some(release) = self.selected_release.clone() {
                    println!("Editing release: {:?}", release);
                    return self
                        .release_widget
                        .update(release_widget::Message::SetSelectedRelease(release))
                        .map(Message::ReleaseWidget);
                }
                Task::none()
            }
            Message::ReleaseWidget(message) => {
                let update_task = self
                    .release_widget
                    .update(message.clone())
                    .map(Message::ReleaseWidget);

                if let release_widget::Message::ReleaseSubmitted(_) = message {
                    // TODO: update selected release
                }

                update_task
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let release_select_view = self
            .release_select_widget
            .view()
            .map(Message::ReleaseSelect);
        let selected_release_view = self.create_selected_release_view();
        let release_view = self.release_widget.view().map(Message::ReleaseWidget);
        column![release_select_view, selected_release_view, release_view].into()
    }

    fn create_selected_release_view(&self) -> iced::Element<Message> {
        if let Some(release) = &self.selected_release {
            let release_name_field = text!("Release Name: {}", release.name);
            let software_titles_field = text!("Software Titles: {:?}", release.software_titles);
            let system_names_field = text!("Systems: {:?}", release.systems);
            let edit_button = button("Edit").on_press(Message::StartEditRelease);

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
