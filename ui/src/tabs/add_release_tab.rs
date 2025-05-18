use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use iced::{
    widget::{button, column, text},
    Task,
};
use service::{
    error::Error as ServiceError, view_model_service::ViewModelService,
    view_models::ReleaseViewModel,
};

use crate::widgets::{
    file_select_widget,
    files_widget::{self, FilesWidget},
    release_select_widget::{self, ReleaseSelectWidget},
    software_title_select_widget,
    software_titles_widget::{self, SoftwareTitlesWidget},
    system_select_widget,
    systems_widget::{self, SystemsWidget},
};

pub struct AddReleaseTab {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,

    // TODO: move these to add_or_edit_release_widget >>
    systems_widget: SystemsWidget,
    selected_system_ids: Vec<i64>,
    software_titles_widget: SoftwareTitlesWidget,
    selected_software_title_ids: Vec<i64>,
    files_widget: FilesWidget,
    selected_file_ids: Vec<i64>,
    // <<
    release_select_widget: ReleaseSelectWidget,
    selected_release: Option<ReleaseViewModel>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Systems(systems_widget::Message),
    SoftwareTitles(software_titles_widget::Message),
    Files(files_widget::Message),
    Submit,
    ReleaseSubmitted(Result<i64, Error>),
    ReleaseSelect(release_select_widget::Message),
    ReleaseFetched(Result<ReleaseViewModel, ServiceError>),
    StartEditRelease,
}

impl AddReleaseTab {
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

        let (release_select_widget, release_select_task) =
            ReleaseSelectWidget::new(Arc::clone(&view_model_service));

        let combined_task = Task::batch(vec![
            systems_task.map(Message::Systems),
            software_titles_task.map(Message::SoftwareTitles),
            files_task.map(Message::Files),
            release_select_task.map(Message::ReleaseSelect),
        ]);

        (
            Self {
                repositories,
                view_model_service,
                selected_system_ids: vec![],
                systems_widget,
                software_titles_widget,
                selected_software_title_ids: vec![],
                files_widget,
                selected_file_ids: vec![],
                release_select_widget,
                selected_release: None,
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
                        println!("AddReleaseTab: System selected: {:?}", system);
                        self.selected_system_ids.push(system.id);
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
                        println!(
                            "AddReleaseTab: Software title selected: {:?}",
                            software_title
                        );
                        self.selected_software_title_ids.push(software_title.id);
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
                        println!("AddReleaseTab: File selected: {:?}", file);
                        self.selected_file_ids.push(file.id);
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
                println!("AddReleaseTab: Submit button pressed");
                let repositories = Arc::clone(&self.repositories);
                let software_title_ids = self.selected_software_title_ids.clone();
                let file_set_ids = self.selected_file_ids.clone();
                let system_ids = self.selected_system_ids.clone();
                Task::perform(
                    async move {
                        repositories
                            .get_release_repository()
                            .add_release_full(
                                "".to_string(),
                                software_title_ids,
                                file_set_ids,
                                system_ids,
                            )
                            .await
                    },
                    Message::ReleaseSubmitted,
                )
            }
            Message::ReleaseSubmitted(result) => match result {
                Ok(id) => {
                    // TODO
                    print!("Release {} submitted", id);
                    Task::none()
                }
                Err(err) => {
                    eprintln!("Error submitting file: {}", err);
                    Task::none()
                }
            },
            Message::ReleaseSelect(message) => {
                let update_task = self
                    .release_select_widget
                    .update(message.clone())
                    .map(Message::ReleaseSelect);

                if let release_select_widget::Message::ReleaseSelected(release) = message.clone() {
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let fetch_selected_release_task = Task::perform(
                        async move { view_model_service.get_release_view_model(release.id).await },
                        Message::ReleaseFetched,
                    );
                    let combined_task = Task::batch(vec![update_task, fetch_selected_release_task]);
                    return combined_task;
                }

                update_task
            }
            Message::ReleaseFetched(result) => match result {
                Ok(release_view_model) => {
                    self.selected_release = Some(release_view_model);
                    Task::none()
                }
                Err(err) => {
                    eprintln!("Error fetching release: {}", err);
                    Task::none()
                }
            },
            Message::StartEditRelease => {
                if let Some(release) = &self.selected_release {
                    // TODO: set all the fields with the release data
                    println!("Editing release: {:?}", release);
                }
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let release_select_view = self
            .release_select_widget
            .view()
            .map(Message::ReleaseSelect);
        let selected_release_view = self.create_selected_release_view();
        let systems_view = self.systems_widget.view().map(Message::Systems);
        let software_titles_view = self
            .software_titles_widget
            .view()
            .map(Message::SoftwareTitles);
        let files_view = self.files_widget.view().map(Message::Files);
        let submit_button = button("Submit").on_press(Message::Submit);
        column![
            release_select_view,
            selected_release_view,
            software_titles_view,
            systems_view,
            files_view,
            submit_button
        ]
        .into()
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
