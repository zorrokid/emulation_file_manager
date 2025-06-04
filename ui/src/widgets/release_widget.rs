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
    file_add_widget::FileAddWidgetMessage,
    file_select_widget::{self, FileSelectWidgetMessage},
    files_widget::{self, FilesWidget, FilesWidgetMessage},
    software_title_select_widget,
    software_titles_widget::{self, SoftwareTitlesWidget, SoftwareTitlesWidgetMessage},
    system_select_widget,
    systems_widget::{self, SystemWidgetMessage, SystemsWidget},
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
pub enum ReleaseWidgetMessage {
    // child messages
    SoftwareTitlesWidget(SoftwareTitlesWidgetMessage),
    Systems(SystemWidgetMessage),
    FilesWidget(FilesWidgetMessage),
    ClearRelease,
    // local messages
    Submit,
    ReleaseSubmitted(Result<i64, Error>),
    SetEditMode,
    Cancel,
    SetSelectedRelease(ReleaseViewModel),
    ReleaseNameChanged(String),
    ReleaseWasUpdated(ReleaseListModel),
}

impl ReleaseWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<ReleaseWidgetMessage>) {
        // TODO fetch data once opened
        let (systems_widget, systems_task) =
            SystemsWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));
        let (software_titles_widget, software_titles_task) =
            SoftwareTitlesWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        let (files_widget, files_task) =
            FilesWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        let combined_task = Task::batch(vec![
            systems_task.map(ReleaseWidgetMessage::Systems),
            software_titles_task.map(ReleaseWidgetMessage::SoftwareTitlesWidget),
            files_task.map(ReleaseWidgetMessage::FilesWidget),
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

    pub fn update(&mut self, message: ReleaseWidgetMessage) -> Task<ReleaseWidgetMessage> {
        match message {
            ReleaseWidgetMessage::Systems(message) => {
                match &message {
                    systems_widget::SystemWidgetMessage::SystemSelect(
                        system_select_widget::SystemSelectWidgetMessage::SystemSelected(system),
                    ) => {
                        if !self.selected_system_ids.contains(&system.id) {
                            self.selected_system_ids.push(system.id);
                        }
                    }
                    systems_widget::SystemWidgetMessage::RemoveSystem(system_id) => {
                        println!("AddReleaseTab: System removed: {:?}", system_id);
                        self.selected_system_ids.retain(|&id| id != *system_id);
                    }
                    _ => {}
                }
                self.systems_widget
                    .update(message)
                    .map(ReleaseWidgetMessage::Systems)
            }
            ReleaseWidgetMessage::SoftwareTitlesWidget(message) => {
                match &message {
                    software_titles_widget::SoftwareTitlesWidgetMessage::SoftwareTitleSelectWidget(
                        software_title_select_widget::SoftwareTitleSelectWidgetMessage::SoftwareTitleSelected(
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
                    software_titles_widget::SoftwareTitlesWidgetMessage::RemoveSoftwareTitle(software_title_id) => {
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
                    .map(ReleaseWidgetMessage::SoftwareTitlesWidget)
            }
            ReleaseWidgetMessage::FilesWidget(message) => {
                match &message {
                    FilesWidgetMessage::FileSelectWidget(
                        FileSelectWidgetMessage::FileSelected(file),
                    ) => {
                        if !self.selected_file_ids.contains(&file.id) {
                            self.selected_file_ids.push(file.id);
                        }
                    }
                    FilesWidgetMessage::RemoveFile(file_id) => {
                        self.selected_file_ids.retain(|&id| id != *file_id);
                    }
                    // TODO: do this also for software titles and systems
                    FilesWidgetMessage::FileAddWidget(FileAddWidgetMessage::FileSetAdded(
                        list_model,
                    )) => {
                        if !self.selected_file_ids.contains(&list_model.id) {
                            self.selected_file_ids.push(list_model.id);
                        }
                    }
                    _ => {}
                }

                self.files_widget
                    .update(message)
                    .map(ReleaseWidgetMessage::FilesWidget)
            }
            ReleaseWidgetMessage::Submit => {
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
                        ReleaseWidgetMessage::ReleaseSubmitted,
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
                        ReleaseWidgetMessage::ReleaseSubmitted,
                    )
                }
            }
            ReleaseWidgetMessage::ReleaseSubmitted(result) => match result {
                Ok(id) => {
                    // TODO
                    print!("Release {} submitted", id);
                    self.is_open = false;
                    Task::done(ReleaseWidgetMessage::ReleaseWasUpdated(ReleaseListModel {
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
            ReleaseWidgetMessage::SetEditMode => {
                self.is_open = true;
                Task::batch(vec![
                    self.systems_widget
                        .update(systems_widget::SystemWidgetMessage::StartEditMode(None))
                        .map(ReleaseWidgetMessage::Systems),
                    self.software_titles_widget
                        .update(software_titles_widget::SoftwareTitlesWidgetMessage::StartEditMode)
                        .map(ReleaseWidgetMessage::SoftwareTitlesWidget),
                    self.files_widget
                        .update(files_widget::FilesWidgetMessage::StartEditMode)
                        .map(ReleaseWidgetMessage::FilesWidget),
                ])
            }
            ReleaseWidgetMessage::Cancel => {
                self.is_open = false;
                self.selected_release = None;
                self.selected_system_ids.clear();
                self.selected_software_title_ids.clear();
                self.selected_file_ids.clear();
                self.release_name.clear();
                Task::batch(vec![
                    self.systems_widget
                        .update(systems_widget::SystemWidgetMessage::Reset)
                        .map(ReleaseWidgetMessage::Systems),
                    self.software_titles_widget
                        .update(software_titles_widget::SoftwareTitlesWidgetMessage::Reset)
                        .map(ReleaseWidgetMessage::SoftwareTitlesWidget),
                    self.files_widget
                        .update(files_widget::FilesWidgetMessage::Reset)
                        .map(ReleaseWidgetMessage::FilesWidget),
                ])
            }
            ReleaseWidgetMessage::SetSelectedRelease(release) => {
                self.is_open = true;
                self.selected_release = Some(release.id);
                let file_set_ids: Vec<i64> = release.file_sets.iter().map(|fs| fs.id).collect();
                self.selected_file_ids = file_set_ids.clone();
                let files_task = self
                    .files_widget
                    .update(files_widget::FilesWidgetMessage::SetSelectedFileIds(
                        file_set_ids,
                    ))
                    .map(ReleaseWidgetMessage::FilesWidget);

                let software_title_ids: Vec<i64> =
                    release.software_titles.iter().map(|st| st.id).collect();
                self.selected_software_title_ids = software_title_ids.clone();
                let software_titles_task = self
                    .software_titles_widget
                    .update(
                        software_titles_widget::SoftwareTitlesWidgetMessage::SetSelectedSoftwareTitleIds(
                            software_title_ids,
                        ),
                    )
                    .map(ReleaseWidgetMessage::SoftwareTitlesWidget);

                let system_ids: Vec<i64> = release.systems.iter().map(|s| s.id).collect();
                // TODO: what if systems widget would emit SystemSelected message for each system
                // (or it could emit a new SystemsSelected message with all selected systems),
                // so this wouldn't need to be set here?
                self.selected_system_ids = system_ids.clone();
                let systems_task = self
                    .systems_widget
                    .update(systems_widget::SystemWidgetMessage::SetSelectedSystemIds(
                        system_ids,
                    ))
                    .map(ReleaseWidgetMessage::Systems);

                self.release_name = release.name.clone();

                Task::batch(vec![software_titles_task, systems_task, files_task])
            }
            ReleaseWidgetMessage::ReleaseNameChanged(name) => {
                self.release_name = name;
                Task::none()
            }
            ReleaseWidgetMessage::ClearRelease => {
                self.is_open = false;
                self.selected_release = None;
                self.selected_system_ids.clear();
                self.selected_software_title_ids.clear();
                self.selected_file_ids.clear();
                self.release_name.clear();
                Task::batch(vec![
                    self.systems_widget
                        .update(systems_widget::SystemWidgetMessage::Reset)
                        .map(ReleaseWidgetMessage::Systems),
                    self.software_titles_widget
                        .update(software_titles_widget::SoftwareTitlesWidgetMessage::Reset)
                        .map(ReleaseWidgetMessage::SoftwareTitlesWidget),
                    self.files_widget
                        .update(files_widget::FilesWidgetMessage::Reset)
                        .map(ReleaseWidgetMessage::FilesWidget),
                ])
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<ReleaseWidgetMessage> {
        let release_view = if self.is_open {
            self.create_release_view()
        } else {
            Column::new().push(button("Add release").on_press(ReleaseWidgetMessage::SetEditMode))
        };
        Container::new(release_view)
            .style(container::bordered_box)
            .into()
    }

    fn create_release_view(&self) -> Column<ReleaseWidgetMessage> {
        let cancel_button_text = if self.selected_release.is_some() {
            "Cancel edit release"
        } else {
            "Cancel add release"
        };
        let cancel_add_emulator_system_button =
            button(cancel_button_text).on_press(ReleaseWidgetMessage::Cancel);

        let systems_view = self
            .systems_widget
            .view(&self.selected_system_ids)
            .map(ReleaseWidgetMessage::Systems);
        let software_titles_view = self
            .software_titles_widget
            .view(&self.selected_software_title_ids)
            .map(ReleaseWidgetMessage::SoftwareTitlesWidget);
        let files_view = self
            .files_widget
            .view(&self.selected_file_ids)
            .map(ReleaseWidgetMessage::FilesWidget);

        let name_input = text_input("Release name", &self.release_name)
            .on_input(ReleaseWidgetMessage::ReleaseNameChanged);

        let submit_button = button("Submit").on_press_maybe(
            (!self.selected_system_ids.is_empty()
                && !self.selected_software_title_ids.is_empty()
                && !self.selected_file_ids.is_empty())
            .then_some(ReleaseWidgetMessage::Submit),
        );
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
