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
    file_add_widget::{self, FileAddWidget, FileAddWidgetMessage},
    file_select_widget::{self, FileSelectWidget, FileSelectWidgetMessage},
};

pub struct FilesWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    files: Vec<FileSetListModel>,
    files_widget: FileSelectWidget,
    add_file_widget: OnceCell<FileAddWidget>,
    // TODO: selected files are also maintained in parent widget!
    selected_file_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub enum FilesWidgetMessage {
    // child messages
    FileAddWidget(FileAddWidgetMessage),
    FileSelectWidget(FileSelectWidgetMessage),
    // local messages
    FilesFetched(Result<Vec<FileSetListModel>, Error>),
    FileAdded(Result<i64, DatabaseError>),
    RemoveFile(i64),
    SettingsFetched(Result<Settings, Error>),
    SetSelectedFileIds(Vec<i64>),
}

impl FilesWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<FilesWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_files_task = Task::perform(
            async move { view_model_service_clone.get_file_set_list_models().await },
            FilesWidgetMessage::FilesFetched,
        );

        let view_model_service_clone = Arc::clone(&view_model_service);
        let fetch_settings_task = Task::perform(
            async move { view_model_service_clone.get_settings().await },
            FilesWidgetMessage::SettingsFetched,
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

    pub fn update(&mut self, message: FilesWidgetMessage) -> Task<FilesWidgetMessage> {
        match message {
            FilesWidgetMessage::FilesFetched(result) => match result {
                Ok(files) => {
                    self.files = files;
                    self.files_widget
                        .update(file_select_widget::FileSelectWidgetMessage::SetFiles(
                            self.files.clone(),
                        ))
                        .map(FilesWidgetMessage::FileSelectWidget)
                }
                Err(error) => {
                    eprint!("Error when fetching files: {}", error);
                    Task::none()
                }
            },
            FilesWidgetMessage::SettingsFetched(result) => match result {
                Ok(settings) => {
                    let collection_root_dir = settings.collection_root_dir.clone();
                    let repositories = Arc::clone(&self.repositories);
                    self.add_file_widget
                        .set(FileAddWidget::new(collection_root_dir, repositories))
                        .unwrap_or_else(|_| {
                            panic!("Failed to set add file widget, already set?");
                        });
                    Task::none()
                }

                Err(error) => {
                    eprint!("Error when fetching settings: {}", error);
                    Task::none()
                }
            },
            FilesWidgetMessage::FileAddWidget(message) => {
                if let file_add_widget::FileAddWidgetMessage::FileSetAdded(list_model) = &message {
                    self.selected_file_ids.push(list_model.id);
                    self.files.push(list_model.clone());
                }
                self.add_file_widget
                    .get_mut()
                    .expect("Add file widget not initialized")
                    .update(message)
                    .map(FilesWidgetMessage::FileAddWidget)
            }
            FilesWidgetMessage::FileSelectWidget(message) => {
                if let file_select_widget::FileSelectWidgetMessage::FileSelected(file) = &message {
                    if !self.selected_file_ids.contains(&file.id) {
                        self.selected_file_ids.push(file.id);
                    }
                }
                self.files_widget
                    .update(message)
                    .map(FilesWidgetMessage::FileSelectWidget)
            }
            FilesWidgetMessage::FileAdded(result) => match result {
                Ok(_) => {
                    let service = Arc::clone(&self.view_model_service);
                    Task::perform(
                        // TODO: get filtered subset of file sets
                        async move { service.get_file_set_list_models().await },
                        FilesWidgetMessage::FilesFetched,
                    )
                }
                Err(error) => {
                    eprint!("Error when adding file: {}", error);
                    Task::none()
                }
            },
            FilesWidgetMessage::RemoveFile(id) => {
                self.selected_file_ids.retain(|&file_id| file_id != id);
                Task::none()
            }
            FilesWidgetMessage::SetSelectedFileIds(ids) => {
                self.selected_file_ids = ids;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<FilesWidgetMessage> {
        let add_file_view = self
            .add_file_widget
            .get()
            .expect("AddFileWidget not initialized")
            .view()
            .map(FilesWidgetMessage::FileAddWidget);
        let files_view = self
            .files_widget
            .view()
            .map(FilesWidgetMessage::FileSelectWidget);
        let selected_files_list = self.create_selected_files_list();
        column![add_file_view, files_view, selected_files_list].into()
    }

    fn create_selected_files_list(&self) -> iced::Element<FilesWidgetMessage> {
        let selected_files = self
            .selected_file_ids
            .iter()
            .map(|id| {
                let file = self
                    .files
                    .iter()
                    .find(|file| file.id == *id)
                    .unwrap_or_else(|| panic!("File with id {} not found", id)); // <== TODO:
                                                                                 // handle error
                let remove_button = button("Remove").on_press(FilesWidgetMessage::RemoveFile(*id));
                row![
                    text!("{}", file.file_set_name.clone()).width(200.0),
                    remove_button
                ]
                .spacing(DEFAULT_SPACING)
                .padding(crate::defaults::DEFAULT_PADDING / 2.0)
                .into()
            })
            .collect::<Vec<Element<FilesWidgetMessage>>>();

        Column::with_children(selected_files).into()
    }
}
