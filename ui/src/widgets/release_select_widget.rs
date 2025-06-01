use std::sync::Arc;

use iced::{
    alignment::Vertical,
    widget::{button, pick_list, row, text},
    Task,
};
use service::{
    error::Error,
    view_model_service::{ReleaseFilter, ViewModelService},
    view_models::ReleaseListModel,
};

use crate::defaults::{DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_SPACING};

pub struct ReleaseSelectWidget {
    releases: Vec<ReleaseListModel>,
    selected_release: Option<ReleaseListModel>,
    view_model_service: Arc<ViewModelService>,
}

#[derive(Debug, Clone)]
pub enum ReleaseSelectWidgetMessage {
    SetReleaseSelected(i64),
    ClearSelectedRelease,
    // local messages
    ReleaseSelected(ReleaseListModel),
    SetReleases(Vec<ReleaseListModel>),
    ReleasesFetched(Result<Vec<ReleaseListModel>, Error>),
    ClearSelection,
    SetFilters(ReleaseFilter),
}

impl ReleaseSelectWidget {
    pub fn new(view_model_service: Arc<ViewModelService>) -> Self {
        //let view_model_service_clone = Arc::clone(&view_model_service);

        // TODO: initially do not fetch releases, only when parent filter sets the filters, fetch
        // based on those filters
        // - add SetFilters message with filters struct
        /*let fetch_releases_task = Task::perform(
            async move { view_model_service_clone.get_release_list_models().await },
            ReleaseSelectWidgetMessage::ReleasesFetched,
        );*/

        Self {
            releases: vec![],
            selected_release: None,
            view_model_service,
        }
    }

    pub fn update(
        &mut self,
        message: ReleaseSelectWidgetMessage,
    ) -> Task<ReleaseSelectWidgetMessage> {
        match message {
            ReleaseSelectWidgetMessage::ReleaseSelected(release) => {
                self.selected_release = Some(release.clone());
                if let Some(release) = &self.selected_release {
                    Task::done(ReleaseSelectWidgetMessage::SetReleaseSelected(release.id))
                } else {
                    Task::none()
                }
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
            ReleaseSelectWidgetMessage::ClearSelection => {
                self.selected_release = None;
                Task::done(ReleaseSelectWidgetMessage::ClearSelectedRelease)
            }
            ReleaseSelectWidgetMessage::SetFilters(filters) => {
                println!("Setting filters: {:?}", filters);
                let view_model_service_clone = Arc::clone(&self.view_model_service);
                let filters = filters.clone();
                Task::perform(
                    async move {
                        view_model_service_clone
                            .get_release_list_models(filters)
                            .await
                    },
                    ReleaseSelectWidgetMessage::ReleasesFetched,
                )
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<ReleaseSelectWidgetMessage> {
        let release_select = pick_list(
            self.releases.as_slice(),
            self.selected_release.clone(),
            ReleaseSelectWidgetMessage::ReleaseSelected,
        );
        let label = text!("Select release").width(DEFAULT_LABEL_WIDTH);
        let clear_filter_button = button("Clear")
            .on_press(ReleaseSelectWidgetMessage::ClearSelection)
            .width(iced::Length::Shrink);
        row![label, release_select, clear_filter_button]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
