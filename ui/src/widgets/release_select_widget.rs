use std::sync::Arc;

use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::{error::Error, view_model_service::ViewModelService, view_models::ReleaseListModel};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

pub struct ReleaseSelectWidget {
    releases: Vec<ReleaseListModel>,
    selected_release: Option<ReleaseListModel>,
    view_model_service: Arc<ViewModelService>,
}

#[derive(Debug, Clone)]
pub enum ReleaseSelectWidgetMessage {
    ReleaseSelected(ReleaseListModel),
    SetReleases(Vec<ReleaseListModel>),
    ReleasesFetched(Result<Vec<ReleaseListModel>, Error>),
    SetReleaseSelected(i64),
}

impl ReleaseSelectWidget {
    pub fn new(
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<ReleaseSelectWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_releases_task = Task::perform(
            async move { view_model_service_clone.get_release_list_models().await },
            ReleaseSelectWidgetMessage::ReleasesFetched,
        );

        (
            Self {
                releases: vec![],
                selected_release: None,
                view_model_service,
            },
            fetch_releases_task,
        )
    }

    pub fn update(
        &mut self,
        message: ReleaseSelectWidgetMessage,
    ) -> Task<ReleaseSelectWidgetMessage> {
        match message {
            ReleaseSelectWidgetMessage::ReleaseSelected(release) => {
                self.selected_release = Some(release.clone());
                Task::done(ReleaseSelectWidgetMessage::SetReleaseSelected(release.id))
            }
            ReleaseSelectWidgetMessage::SetReleases(releases) => {
                self.releases = releases;
                self.selected_release = None;
                Task::none()
            }
            ReleaseSelectWidgetMessage::ReleasesFetched(result) => match result {
                Ok(releases) => {
                    println!("Fetched releases: {:?}", releases);
                    self.releases = releases;
                    Task::none()
                }
                Err(err) => {
                    eprintln!("Error fetching releases: {:?}", err);
                    Task::none()
                }
            },
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<ReleaseSelectWidgetMessage> {
        let release_select = pick_list(
            self.releases.as_slice(),
            self.selected_release.clone(),
            ReleaseSelectWidgetMessage::ReleaseSelected,
        );
        let label = text!("Select release");
        row![label, release_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
