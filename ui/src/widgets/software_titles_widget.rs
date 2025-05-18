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
    software_title_add_widget::{self, SoftwareTitleAddWidget},
    software_title_select_widget::{self, SoftwareTitleSelectWidget},
};

pub struct SoftwareTitlesWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    software_titles: Vec<SoftwareTitleListModel>,
    software_titles_widget: SoftwareTitleSelectWidget,
    add_software_title_widget: SoftwareTitleAddWidget,
    selected_software_title_ids: Vec<i64>,
    is_adding_software_title: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, Error>),
    AddSoftwareTitle(software_title_add_widget::Message),
    SoftwareTitleSelect(software_title_select_widget::Message),
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
    ) -> (Self, Task<Message>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_software_titles_task = Task::perform(
            async move {
                view_model_service_clone
                    .get_software_title_list_models()
                    .await
            },
            Message::SoftwareTitlesFetched,
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SoftwareTitlesFetched(result) => match result {
                Ok(software_titles) => {
                    self.software_titles = software_titles;
                    self.software_titles_widget
                        .update(software_title_select_widget::Message::SetSoftwareTitles(
                            self.software_titles.clone(),
                        ))
                        .map(Message::SoftwareTitleSelect)
                }
                Err(error) => {
                    eprint!("Error when fetching software_titles: {}", error);
                    Task::none()
                }
            },
            Message::AddSoftwareTitle(message) => match message {
                software_title_add_widget::Message::AddSoftwareTitle(name) => {
                    let repo = Arc::clone(&self.repositories);
                    Task::perform(
                        async move {
                            repo.get_software_title_repository()
                                .add_software_title(name, None)
                                .await
                        },
                        Message::SoftwareTitleAdded,
                    )
                }
                software_title_add_widget::Message::UpdateSoftwareTitle(id, name) => {
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
                        Message::SoftwareTitleUpdated,
                    )
                }
                _ => self
                    .add_software_title_widget
                    .update(message)
                    .map(Message::AddSoftwareTitle),
            },
            Message::SoftwareTitleSelect(message) => {
                if let software_title_select_widget::Message::SoftwareTitleSelected(
                    software_title,
                ) = message
                {
                    self.selected_software_title_ids.push(software_title.id);
                    Task::none()
                } else {
                    Task::none()
                }
            }
            Message::SoftwareTitleAdded(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    self.is_adding_software_title = false;
                    // TODO no need to fetch is we update the newly added software title with list
                    // model
                    Task::perform(
                        async move { service.get_software_title_list_models().await },
                        Message::SoftwareTitlesFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when adding software_title: {}", error);
                    Task::none()
                }
            },
            Message::SoftwareTitleUpdated(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    self.is_adding_software_title = false;
                    // TODO no need to fetch is we update the newly added software title with list
                    // model
                    Task::perform(
                        async move { service.get_software_title_list_models().await },
                        Message::SoftwareTitlesFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when updating software_title: {}", error);
                    Task::none()
                }
            },
            Message::RemoveSoftwareTitle(id) => {
                self.selected_software_title_ids
                    .retain(|&software_title_id| software_title_id != id);
                Task::none()
            }
            Message::SetSelectedSoftwareTitleIds(ids) => {
                self.selected_software_title_ids = ids;
                Task::none()
            }
            Message::ToggleIsAddingSoftwareTitle => {
                self.is_adding_software_title = !self.is_adding_software_title;
                Task::none()
            }
            Message::StartEditSoftwareTitle(id) => {
                let software_title = self
                    .software_titles
                    .iter()
                    .find(|software_title| software_title.id == id)
                    .unwrap_or_else(|| panic!("SoftwareTitle with id {} not found", id));
                self.is_adding_software_title = true;
                let name = software_title.name.clone();
                self.add_software_title_widget
                    .update(software_title_add_widget::Message::SetEditSoftwareTitle(
                        id, name,
                    ))
                    .map(Message::AddSoftwareTitle)
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let add_view: Element<Message> = if self.is_adding_software_title {
            let add_software_title_view = self
                .add_software_title_widget
                .view()
                .map(Message::AddSoftwareTitle);
            let cancel_button = button("Cancel").on_press(Message::ToggleIsAddingSoftwareTitle);
            column![cancel_button, add_software_title_view].into()
        } else {
            button("Add Software Title")
                .on_press(Message::ToggleIsAddingSoftwareTitle)
                .into()
        };

        let software_titles_view = self
            .software_titles_widget
            .view()
            .map(Message::SoftwareTitleSelect);
        let selected_software_titles_list = self.create_selected_software_titles_list();
        column![
            software_titles_view,
            selected_software_titles_list,
            add_view,
        ]
        .into()
    }

    fn create_selected_software_titles_list(&self) -> iced::Element<Message> {
        let selected_software_titles = self
            .selected_software_title_ids
            .iter()
            .map(|id| {
                let software_title = self
                    .software_titles
                    .iter()
                    .find(|software_title| software_title.id == *id)
                    .unwrap_or_else(|| panic!("SoftwareTitle with id {} not found", id));
                let remove_button = button("Remove").on_press(Message::RemoveSoftwareTitle(*id));
                let edit_button = button("Edit").on_press(Message::StartEditSoftwareTitle(*id));
                row![
                    text!("{}", software_title.name.clone()).width(200.0),
                    edit_button,
                    remove_button
                ]
                .spacing(DEFAULT_SPACING)
                .padding(crate::defaults::DEFAULT_PADDING / 2.0)
                .into()
            })
            .collect::<Vec<Element<Message>>>();

        Column::with_children(selected_software_titles).into()
    }
}
