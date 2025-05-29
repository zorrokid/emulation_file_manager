use std::sync::Arc;

use database::{
    database_error::Error as DatabaseError, models::SoftwareTitle,
    repository_manager::RepositoryManager,
};
use iced::{
    widget::{button, column, row, text, Column},
    Element, Task,
};
use service::{
    error::Error, view_model_service::ViewModelService, view_models::SoftwareTitleListModel,
};

use crate::defaults::DEFAULT_SPACING;

use super::{
    software_title_add_widget::{self, SoftwareTitleAddWidget, SoftwareTitleAddWidgetMessage},
    software_title_select_widget::{
        self, SoftwareTitleSelectWidget, SoftwareTitleSelectWidgetMessage,
    },
};

pub struct SoftwareTitlesWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    software_titles: Vec<SoftwareTitleListModel>,
    software_titles_widget: SoftwareTitleSelectWidget,
    add_software_title_widget: SoftwareTitleAddWidget,
    // TODO: selected software titles are also maintained in parent widget!
    selected_software_title_ids: Vec<i64>,
    is_adding_software_title: bool,
}

#[derive(Debug, Clone)]
pub enum SoftwareTitlesWidgetMessage {
    // child messages
    SoftwareTitleAddWidget(SoftwareTitleAddWidgetMessage),
    SoftwareTitleSelectWidget(SoftwareTitleSelectWidgetMessage),
    // local messages
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, Error>),
    SoftwareTitleAdded(Result<i64, DatabaseError>),
    SoftwareTitleUpdated(Result<i64, DatabaseError>),
    RemoveSoftwareTitle(i64),
    StartEditSoftwareTitle(i64),
    SetSelectedSoftwareTitleIds(Vec<i64>),
    ToggleIsAddingSoftwareTitle,
}

impl SoftwareTitlesWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<SoftwareTitlesWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_software_titles_task = Task::perform(
            async move {
                view_model_service_clone
                    .get_software_title_list_models()
                    .await
            },
            SoftwareTitlesWidgetMessage::SoftwareTitlesFetched,
        );

        (
            Self {
                repositories,
                view_model_service,
                software_titles: vec![],
                software_titles_widget: SoftwareTitleSelectWidget::new(),
                add_software_title_widget: SoftwareTitleAddWidget::new(),
                selected_software_title_ids: vec![],
                is_adding_software_title: false,
            },
            fetch_software_titles_task,
        )
    }

    pub fn update(
        &mut self,
        message: SoftwareTitlesWidgetMessage,
    ) -> Task<SoftwareTitlesWidgetMessage> {
        match message {
            SoftwareTitlesWidgetMessage::SoftwareTitlesFetched(result) => match result {
                Ok(software_titles) => {
                    self.software_titles = software_titles;
                    self.software_titles_widget
                        .update(software_title_select_widget::SoftwareTitleSelectWidgetMessage::SetSoftwareTitles(
                            self.software_titles.clone(),
                        ))
                        .map(SoftwareTitlesWidgetMessage::SoftwareTitleSelectWidget)
                }
                Err(error) => {
                    eprint!("Error when fetching software_titlejk {}", error);
                    Task::none()
                }
            },
            SoftwareTitlesWidgetMessage::SoftwareTitleAddWidget(message) => match message {
                software_title_add_widget::SoftwareTitleAddWidgetMessage::AddSoftwareTitle(
                    name,
                ) => {
                    let repo = Arc::clone(&self.repositories);
                    Task::perform(
                        async move {
                            repo.get_software_title_repository()
                                .add_software_title(name, None)
                                .await
                        },
                        SoftwareTitlesWidgetMessage::SoftwareTitleAdded,
                    )
                }
                software_title_add_widget::SoftwareTitleAddWidgetMessage::UpdateSoftwareTitle(
                    id,
                    name,
                ) => {
                    let repo = Arc::clone(&self.repositories);
                    let software_title = SoftwareTitle {
                        id,
                        name: name.clone(),
                        franchise_id: None,
                    };

                    Task::perform(
                        async move {
                            repo.get_software_title_repository()
                                .update_software_title(software_title)
                                .await
                        },
                        SoftwareTitlesWidgetMessage::SoftwareTitleUpdated,
                    )
                }
                _ => self
                    .add_software_title_widget
                    .update(message)
                    .map(SoftwareTitlesWidgetMessage::SoftwareTitleAddWidget),
            },
            SoftwareTitlesWidgetMessage::SoftwareTitleSelectWidget(message) => {
                if let software_title_select_widget::SoftwareTitleSelectWidgetMessage::SoftwareTitleSelected(
                    software_title,
                ) = message
                {
                    if !self
                        .selected_software_title_ids
                        .contains(&software_title.id)
                    {
                        self.selected_software_title_ids.push(software_title.id);
                    }
                }
                Task::none()
            }
            SoftwareTitlesWidgetMessage::SoftwareTitleAdded(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    self.is_adding_software_title = false;
                    // TODO no need to fetch is we update the newly added software title with list
                    // model
                    Task::perform(
                        async move { service.get_software_title_list_models().await },
                        SoftwareTitlesWidgetMessage::SoftwareTitlesFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when adding software_title: {}", error);
                    Task::none()
                }
            },
            SoftwareTitlesWidgetMessage::SoftwareTitleUpdated(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    self.is_adding_software_title = false;
                    // TODO no need to fetch is we update the newly added software title with list
                    // model
                    Task::perform(
                        async move { service.get_software_title_list_models().await },
                        SoftwareTitlesWidgetMessage::SoftwareTitlesFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when updating software_title: {}", error);
                    Task::none()
                }
            },
            SoftwareTitlesWidgetMessage::RemoveSoftwareTitle(id) => {
                self.selected_software_title_ids
                    .retain(|&software_title_id| software_title_id != id);
                Task::none()
            }
            SoftwareTitlesWidgetMessage::SetSelectedSoftwareTitleIds(ids) => {
                self.selected_software_title_ids = ids;
                Task::none()
            }
            SoftwareTitlesWidgetMessage::ToggleIsAddingSoftwareTitle => {
                self.is_adding_software_title = !self.is_adding_software_title;
                Task::none()
            }
            SoftwareTitlesWidgetMessage::StartEditSoftwareTitle(id) => {
                let software_title = self
                    .software_titles
                    .iter()
                    .find(|software_title| software_title.id == id)
                    .unwrap_or_else(|| panic!("SoftwareTitle with id {} not found", id));
                self.is_adding_software_title = true;
                let name = software_title.name.clone();
                self.add_software_title_widget
                    .update(software_title_add_widget::SoftwareTitleAddWidgetMessage::SetEditSoftwareTitle(
                        id, name,
                    ))
                    .map(SoftwareTitlesWidgetMessage::SoftwareTitleAddWidget)
            }
        }
    }

    pub fn view(&self) -> iced::Element<SoftwareTitlesWidgetMessage> {
        let add_view: Element<SoftwareTitlesWidgetMessage> = if self.is_adding_software_title {
            let add_software_title_view = self
                .add_software_title_widget
                .view()
                .map(SoftwareTitlesWidgetMessage::SoftwareTitleAddWidget);
            let cancel_button =
                button("Cancel").on_press(SoftwareTitlesWidgetMessage::ToggleIsAddingSoftwareTitle);
            column![cancel_button, add_software_title_view].into()
        } else {
            button("Add Software Title")
                .on_press(SoftwareTitlesWidgetMessage::ToggleIsAddingSoftwareTitle)
                .into()
        };

        let software_titles_view = self
            .software_titles_widget
            .view()
            .map(SoftwareTitlesWidgetMessage::SoftwareTitleSelectWidget);
        let selected_software_titles_list = self.create_selected_software_titles_list();
        column![
            software_titles_view,
            selected_software_titles_list,
            add_view,
        ]
        .into()
    }

    fn create_selected_software_titles_list(&self) -> iced::Element<SoftwareTitlesWidgetMessage> {
        let selected_software_titles = self
            .selected_software_title_ids
            .iter()
            .map(|id| {
                let software_title = self
                    .software_titles
                    .iter()
                    .find(|software_title| software_title.id == *id)
                    .unwrap_or_else(|| panic!("SoftwareTitle with id {} not found", id));
                let remove_button = button("Remove")
                    .on_press(SoftwareTitlesWidgetMessage::RemoveSoftwareTitle(*id));
                let edit_button = button("Edit")
                    .on_press(SoftwareTitlesWidgetMessage::StartEditSoftwareTitle(*id));
                row![
                    text!("{}", software_title.name.clone()).width(200.0),
                    edit_button,
                    remove_button
                ]
                .spacing(DEFAULT_SPACING)
                .padding(crate::defaults::DEFAULT_PADDING / 2.0)
                .into()
            })
            .collect::<Vec<Element<SoftwareTitlesWidgetMessage>>>();

        Column::with_children(selected_software_titles).into()
    }
}
