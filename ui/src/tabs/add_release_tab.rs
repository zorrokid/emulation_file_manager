use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{widget::column, Task};
use service::view_model_service::ViewModelService;

use crate::widgets::{
    file_select_widget,
    files_widget::{self, FilesWidget},
    software_title_select_widget,
    software_titles_widget::{self, SoftwareTitlesWidget},
    system_select_widget,
    systems_widget::{self, SystemsWidget},
};

pub struct AddReleaseTab {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems_widget: SystemsWidget,
    selected_system_ids: Vec<i64>,
    software_titles_widget: SoftwareTitlesWidget,
    selected_software_title_ids: Vec<i64>,
    files_widget: FilesWidget,
    selected_file_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Systems(systems_widget::Message),
    SoftwareTitles(software_titles_widget::Message),
    Files(files_widget::Message),
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

        let combined_task = Task::batch(vec![
            systems_task.map(Message::Systems),
            software_titles_task.map(Message::SoftwareTitles),
            files_task.map(Message::Files),
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
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let systems_view = self.systems_widget.view().map(Message::Systems);
        let software_titles_view = self
            .software_titles_widget
            .view()
            .map(Message::SoftwareTitles);
        let files_view = self.files_widget.view().map(Message::Files);
        column![systems_view, software_titles_view, files_view].into()
    }
}
