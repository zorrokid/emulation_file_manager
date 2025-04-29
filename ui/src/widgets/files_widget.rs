use std::{cell::OnceCell, sync::Arc};

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use iced::{
    widget::{button, column, row, text, Column},
    Element, Task,
};
use service::{
    error::Error,
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};

use crate::defaults::DEFAULT_SPACING;

use super::{
    file_add_widget::{self, FileAddWidget},
    file_select_widget::{self, FileSelectWidget},
};

pub struct FilesWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    files: Vec<FileSetListModel>,
    files_widget: FileSelectWidget,
    add_file_widget: OnceCell<FileAddWidget>,
    selected_file_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    FilesFetched(Result<Vec<FileSetListModel>, Error>),
    AddFile(file_add_widget::Message),
    FileSelect(file_select_widget::Message),
    FileAdded(Result<i64, DatabaseError>),
    RemoveFile(i64),
    SettingsFetched(Result<Settings, Error>),
}

impl FilesWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_files_task = Task::perform(
            async move { view_model_service_clone.get_file_set_list_models().await },
            Message::FilesFetched,
        );

        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_settings_task = Task::perform(
            async move { view_model_service_clone.get_settings().await },
            Message::SettingsFetched,
        );

        let combined_task = Task::batch(vec![fetch_files_task, fetch_settings_task]);

        (
            Self {
                repositories,
                view_model_service,
                files: vec![],
                files_widget: FileSelectWidget::new(),
                add_file_widget: OnceCell::new(), // FileAddWidget::new(),
                selected_file_ids: vec![],
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FilesFetched(result) => match result {
                Ok(files) => {
                    self.files = files;
                    self.files_widget
                        .update(file_select_widget::Message::SetFiles(self.files.clone()))
                        .map(Message::FileSelect)
                }
                Err(error) => {
                    eprint!("Error when fetching files: {}", error);
                    Task::none()
                }
            },
            Message::SettingsFetched(result) => match result {
                Ok(settings) => {
                    let collection_root_dir = settings.collection_root_dir.clone();
                    self.add_file_widget
                        .set(FileAddWidget::new(collection_root_dir));
                    Task::none()
                }

                Err(error) => {
                    eprint!("Error when fetching settings: {}", error);
                    Task::none()
                }
            },
            Message::AddFile(message) => {
                let add_file_widget = self
                    .add_file_widget
                    .get_mut()
                    .expect("Add file widget not initialized");
                add_file_widget.update(message).map(Message::AddFile)
            }
            Message::FileSelect(message) => {
                if let file_select_widget::Message::FileSelected(file) = &message {
                    self.selected_file_ids.push(file.id);
                }
                self.files_widget.update(message).map(Message::FileSelect)
            }
            Message::FileAdded(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    Task::perform(
                        // TODO: get filtered subset of file sets
                        async move { service.get_file_set_list_models().await },
                        Message::FilesFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when adding file: {}", error);
                    Task::none()
                }
            },
            Message::RemoveFile(id) => {
                self.selected_file_ids.retain(|&file_id| file_id != id);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let add_file_view = self
            .add_file_widget
            .get()
            .expect("AddFileWidget not initialized")
            .view()
            .map(Message::AddFile);
        let files_view = self.files_widget.view().map(Message::FileSelect);
        let selected_files_list = self.create_selected_files_list();
        column![add_file_view, files_view, selected_files_list].into()
    }

    fn create_selected_files_list(&self) -> iced::Element<Message> {
        let selected_files = self
            .selected_file_ids
            .iter()
            .map(|id| {
                let file = self
                    .files
                    .iter()
                    .find(|file| file.id == *id)
                    .unwrap_or_else(|| panic!("File with id {} not found", id));
                let remove_button = button("Remove").on_press(Message::RemoveFile(*id));
                row![
                    text!("{}", file.file_set_name.clone()).width(200.0),
                    remove_button
                ]
                .spacing(DEFAULT_SPACING)
                .padding(crate::defaults::DEFAULT_PADDING / 2.0)
                .into()
            })
            .collect::<Vec<Element<Message>>>();

        Column::with_children(selected_files).into()
    }
}
