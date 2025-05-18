use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use iced::{
    widget::{button, column, container, text_input, Column, Container},
    Element, Task,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{ReleaseListModel, ReleaseViewModel},
};

use super::{
    file_select_widget,
    files_widget::{self, FilesWidget},
    software_title_select_widget,
    software_titles_widget::{self, SoftwareTitlesWidget},
    system_select_widget,
    systems_widget::{self, SystemsWidget},
};

pub struct ReleaseWidget {
    systems_widget: SystemsWidget,
    selected_system_ids: Vec<i64>,
    software_titles_widget: SoftwareTitlesWidget,
    selected_software_title_ids: Vec<i64>,
    files_widget: FilesWidget,
    selected_file_ids: Vec<i64>,
    repositories: Arc<RepositoryManager>,
    selected_release: Option<i64>,
    is_open: bool,
    release_name: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Systems(systems_widget::Message),
    SoftwareTitles(software_titles_widget::Message),
    Files(files_widget::Message),
    Submit,
    ReleaseSubmitted(Result<i64, Error>),
    ToggleOpen,
    SetSelectedRelease(ReleaseViewModel),
    ReleaseNameChanged(String),
    ReleaseWasUpdated(ReleaseListModel),
}

impl ReleaseWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let (systems_widget, systems_task) =
            SystemsWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));
        let (software_titles_widget, software_titles_task) =
            SoftwareTitlesWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        let (files_widget, files_task) =
            FilesWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        let combined_task = Task::batch(vec![
            systems_task.map(Message::Systems),
            software_titles_task.map(Message::SoftwareTitles),
            files_task.map(Message::Files),
        ]);

        (
            Self {
                systems_widget,
                selected_system_ids: vec![],
                software_titles_widget,
                selected_software_title_ids: vec![],
                files_widget,
                selected_file_ids: vec![],
                repositories,
                is_open: false,
                selected_release: None,
                release_name: "".to_string(),
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Systems(message) => {
                match &message {
                    systems_widget::Message::SystemSelect(
                        system_select_widget::Message::SystemSelected(system),
                    ) => {
                        if !self.selected_system_ids.contains(&system.id) {
                            self.selected_system_ids.push(system.id);
                        }
                    }
                    systems_widget::Message::RemoveSystem(system_id) => {
                        println!("AddReleaseTab: System removed: {:?}", system_id);
                        self.selected_system_ids.retain(|&id| id != *system_id);
                    }
                    _ => {}
                }
                self.systems_widget.update(message).map(Message::Systems)
            }
            Message::SoftwareTitles(message) => {
                match &message {
                    software_titles_widget::Message::SoftwareTitleSelect(
                        software_title_select_widget::Message::SoftwareTitleSelected(
                            software_title,
                        ),
                    ) => {
                        if !self
                            .selected_software_title_ids
                            .contains(&software_title.id)
                        {
                            self.selected_software_title_ids.push(software_title.id);
                        }
                    }
                    software_titles_widget::Message::RemoveSoftwareTitle(software_title_id) => {
                        println!(
                            "AddReleaseTab: Software title removed: {:?}",
                            software_title_id
                        );
                        self.selected_software_title_ids
                            .retain(|&id| id != *software_title_id);
                    }
                    _ => {}
                }

                self.software_titles_widget
                    .update(message)
                    .map(Message::SoftwareTitles)
            }
            Message::Files(message) => {
                match &message {
                    files_widget::Message::FileSelect(
                        file_select_widget::Message::FileSelected(file),
                    ) => {
                        if !self.selected_file_ids.contains(&file.id) {
                            self.selected_file_ids.push(file.id);
                        }
                    }
                    files_widget::Message::RemoveFile(file_id) => {
                        println!("AddReleaseTab: File removed: {:?}", file_id);
                        self.selected_file_ids.retain(|&id| id != *file_id);
                    }
                    _ => {}
                }

                self.files_widget.update(message).map(Message::Files)
            }
            Message::Submit => {
                let repositories = Arc::clone(&self.repositories);
                let software_title_ids = self.selected_software_title_ids.clone();
                let file_set_ids = self.selected_file_ids.clone();
                let system_ids = self.selected_system_ids.clone();
                let name = self.release_name.clone();

                if let Some(release_id) = self.selected_release {
                    Task::perform(
                        async move {
                            repositories
                                .get_release_repository()
                                .update_release_full(
                                    release_id,
                                    name,
                                    software_title_ids,
                                    file_set_ids,
                                    system_ids,
                                )
                                .await
                        },
                        Message::ReleaseSubmitted,
                    )
                } else {
                    Task::perform(
                        async move {
                            repositories
                                .get_release_repository()
                                .add_release_full(
                                    name,
                                    software_title_ids,
                                    file_set_ids,
                                    system_ids,
                                )
                                .await
                        },
                        Message::ReleaseSubmitted,
                    )
                }
            }
            Message::ReleaseSubmitted(result) => match result {
                Ok(id) => {
                    // TODO
                    print!("Release {} submitted", id);
                    self.is_open = false;
                    Task::done(Message::ReleaseWasUpdated(ReleaseListModel {
                        id,
                        name: self.release_name.clone(),
                        system_names: vec![],
                        file_types: vec![],
                    }))
                }
                Err(err) => {
                    eprintln!("Error submitting file: {}", err);
                    Task::none()
                }
            },
            Message::ToggleOpen => {
                self.is_open = !self.is_open;
                Task::none()
            }
            Message::SetSelectedRelease(release) => {
                self.is_open = true;
                self.selected_release = Some(release.id);
                let file_set_ids: Vec<i64> = release.file_sets.iter().map(|fs| fs.id).collect();
                self.selected_file_ids = file_set_ids.clone();
                let files_task = self
                    .files_widget
                    .update(files_widget::Message::SetSelectedFileIds(file_set_ids))
                    .map(Message::Files);

                let software_title_ids: Vec<i64> =
                    release.software_titles.iter().map(|st| st.id).collect();
                self.selected_software_title_ids = software_title_ids.clone();
                let software_titles_task = self
                    .software_titles_widget
                    .update(
                        software_titles_widget::Message::SetSelectedSoftwareTitleIds(
                            software_title_ids,
                        ),
                    )
                    .map(Message::SoftwareTitles);

                let system_ids: Vec<i64> = release.systems.iter().map(|s| s.id).collect();
                // TODO: what if systems widget would emit SystemSelected message for each system
                // (or it could emit a new SystemsSelected message with all selected systems),
                // so this wouldn't need to be set here?
                self.selected_system_ids = system_ids.clone();
                let systems_task = self
                    .systems_widget
                    .update(systems_widget::Message::SetSelectedSystemIds(system_ids))
                    .map(Message::Systems);

                self.release_name = release.name.clone();

                Task::batch(vec![software_titles_task, systems_task, files_task])
            }
            Message::ReleaseNameChanged(name) => {
                self.release_name = name;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<Message> {
        let release_view = if self.is_open {
            self.create_release_view()
        } else {
            Column::new().push(button("Add release").on_press(Message::ToggleOpen))
        };
        Container::new(release_view)
            .style(container::bordered_box)
            .into()
    }

    fn create_release_view(&self) -> Column<Message> {
        let cancel_button_text = if self.selected_release.is_some() {
            "Cancel edit release"
        } else {
            "Cancel add release"
        };
        let cancel_add_emulator_system_button =
            button(cancel_button_text).on_press(Message::ToggleOpen);

        let systems_view = self.systems_widget.view().map(Message::Systems);
        let software_titles_view = self
            .software_titles_widget
            .view()
            .map(Message::SoftwareTitles);
        let files_view = self.files_widget.view().map(Message::Files);

        let name_input =
            text_input("Release name", &self.release_name).on_input(Message::ReleaseNameChanged);

        let submit_button = button("Submit").on_press(Message::Submit);
        column![
            cancel_add_emulator_system_button,
            name_input,
            software_titles_view,
            systems_view,
            files_view,
            submit_button
        ]
    }
}
