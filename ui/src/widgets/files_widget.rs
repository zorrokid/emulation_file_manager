use std::{cell::OnceCell, sync::Arc};

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use iced::{
    widget::{button, column, container, row, text, Column, Container},
    Element, Length, Task,
};
use service::{
    error::Error,
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::{
    file_add_widget::{self, FileAddWidget, FileAddWidgetMessage},
    file_select_widget::{self, FileSelectWidget, FileSelectWidgetMessage},
};

pub struct FilesWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    files: Vec<FileSetListModel>,
    file_select_widget: FileSelectWidget,
    add_file_widget: OnceCell<FileAddWidget>,
    is_edit_mode: bool,
}

#[derive(Debug, Clone)]
pub enum FilesWidgetMessage {
    // parent messages
    Reset,
    StartEditMode,
    // child messages
    FileAddWidget(FileAddWidgetMessage),
    FileSelectWidget(FileSelectWidgetMessage),
    // local messages
    FilesFetched(Result<Vec<FileSetListModel>, Error>),
    FileAdded(Result<i64, DatabaseError>),
    RemoveFile(i64),
    SettingsFetched(Result<Settings, Error>),
    SetSelectedFileIds(Vec<i64>),
    StartAddFile,
    CancelAddFile,
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
                file_select_widget: FileSelectWidget::new(),
                add_file_widget: OnceCell::new(),
                is_edit_mode: false,
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: FilesWidgetMessage) -> Task<FilesWidgetMessage> {
        match message {
            FilesWidgetMessage::FilesFetched(result) => match result {
                Ok(files) => {
                    self.files = files;
                    Task::none()
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
                    self.files.push(list_model.clone());
                }
                self.add_file_widget
                    .get_mut()
                    .expect("Add file widget not initialized")
                    .update(message)
                    .map(FilesWidgetMessage::FileAddWidget)
            }
            FilesWidgetMessage::FileSelectWidget(message) => self
                .file_select_widget
                .update(message)
                .map(FilesWidgetMessage::FileSelectWidget),
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
            FilesWidgetMessage::Reset => self
                .file_select_widget
                .update(file_select_widget::FileSelectWidgetMessage::Reset)
                .map(FilesWidgetMessage::FileSelectWidget),
            FilesWidgetMessage::StartEditMode => {
                let view_model_service_clone = Arc::clone(&self.view_model_service);
                Task::perform(
                    async move { view_model_service_clone.get_file_set_list_models().await },
                    FilesWidgetMessage::FilesFetched,
                )
            }
            FilesWidgetMessage::StartAddFile => {
                self.is_edit_mode = true;
                Task::none()
            }
            FilesWidgetMessage::CancelAddFile => {
                self.is_edit_mode = false;
                self.add_file_widget
                    .get_mut()
                    .expect("Add file widget not initialized")
                    .update(file_add_widget::FileAddWidgetMessage::Reset)
                    .map(FilesWidgetMessage::FileAddWidget)
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self, selected_file_ids: &[i64]) -> iced::Element<FilesWidgetMessage> {
        let add_file_view: Element<FilesWidgetMessage> = if self.is_edit_mode {
            let add_file_view = self
                .add_file_widget
                .get()
                .expect("AddFileWidget not initialized")
                .view()
                .map(FilesWidgetMessage::FileAddWidget);
            let cancel_button = button("Cancel").on_press(FilesWidgetMessage::CancelAddFile);
            column![cancel_button, add_file_view].into()
        } else {
            button("Add File")
                .on_press(FilesWidgetMessage::StartAddFile)
                .into()
        };
        let files_select_view = self
            .file_select_widget
            .view(&self.files)
            .map(FilesWidgetMessage::FileSelectWidget);
        let selected_files_list = self.create_selected_files_list(selected_file_ids);
        let content = column![files_select_view, add_file_view, selected_files_list];
        Container::new(content)
            .style(container::bordered_box)
            .padding(DEFAULT_PADDING)
            .width(Length::Fill)
            .into()
    }

    fn create_selected_files_list(
        &self,
        selected_file_ids: &[i64],
    ) -> iced::Element<FilesWidgetMessage> {
        let selected_files = selected_file_ids
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
