use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
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
}

#[derive(Debug, Clone)]
pub enum Message {
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, Error>),
    AddSoftwareTitle(software_title_add_widget::Message),
    SoftwareTitleSelect(software_title_select_widget::Message),
    SoftwareTitleAdded(Result<i64, DatabaseError>),
    RemoveSoftwareTitle(i64),
    SetSelectedSoftwareTitleIds(Vec<i64>),
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
            Message::AddSoftwareTitle(message) => {
                match self.add_software_title_widget.update(message) {
                    software_title_add_widget::Action::AddSoftwareTitle(name) => {
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
                    software_title_add_widget::Action::None => Task::none(),
                }
            }
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
            Message::RemoveSoftwareTitle(id) => {
                self.selected_software_title_ids
                    .retain(|&software_title_id| software_title_id != id);
                Task::none()
            }
            Message::SetSelectedSoftwareTitleIds(ids) => {
                self.selected_software_title_ids = ids;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let add_software_title_view = self
            .add_software_title_widget
            .view()
            .map(Message::AddSoftwareTitle);
        let software_titles_view = self
            .software_titles_widget
            .view()
            .map(Message::SoftwareTitleSelect);
        let selected_software_titles_list = self.create_selected_software_titles_list();
        column![
            add_software_title_view,
            software_titles_view,
            selected_software_titles_list
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
                row![
                    text!("{}", software_title.name.clone()).width(200.0),
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
