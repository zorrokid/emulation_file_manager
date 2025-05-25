use database::models::FileSetFileInfo;
use iced::{
    widget::{button, column, pick_list, text},
    Element, Task,
};
use service::view_models::{FileSetViewModel, ReleaseViewModel};

pub struct ReleaseViewWidget {
    release: Option<ReleaseViewModel>,
    selected_file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfo>,
}

#[derive(Debug, Clone)]
pub enum ReleaseViewWidgetMessage {
    SetEditRelease(ReleaseViewModel),
    SetRelease(ReleaseViewModel),
    // Local messages
    StartEditRelease,
    SetSelectedFileSet(FileSetViewModel),
    SetSelectedFile(FileSetFileInfo),
}

impl ReleaseViewWidget {
    pub fn new() -> Self {
        ReleaseViewWidget {
            release: None,
            selected_file_set: None,
            selected_file: None,
        }
    }

    pub fn update(&mut self, message: ReleaseViewWidgetMessage) -> Task<ReleaseViewWidgetMessage> {
        match message {
            ReleaseViewWidgetMessage::StartEditRelease => {
                if let Some(release) = &self.release {
                    Task::done(ReleaseViewWidgetMessage::SetEditRelease(release.clone()))
                } else {
                    Task::none()
                }
            }
            ReleaseViewWidgetMessage::SetRelease(release) => {
                self.release = Some(release);
                self.selected_file_set = None;
                self.selected_file = None;
                Task::none()
            }
            ReleaseViewWidgetMessage::SetSelectedFileSet(file_set) => {
                self.selected_file_set = Some(file_set);
                self.selected_file = None;
                Task::none()
            }
            ReleaseViewWidgetMessage::SetSelectedFile(file) => {
                self.selected_file = Some(file);
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<ReleaseViewWidgetMessage> {
        if let Some(release) = &self.release {
            let release_name_field = text!("Release Name: {}", release.name);
            let software_titles_field = text!("Software Titles: {:?}", release.software_titles);
            let system_names_field = text!("Systems: {:?}", release.systems);
            let edit_button = button("Edit").on_press(ReleaseViewWidgetMessage::StartEditRelease);
            let file_sets_select: Element<ReleaseViewWidgetMessage> = pick_list(
                release.file_sets.as_slice(),
                self.selected_file_set.clone(),
                ReleaseViewWidgetMessage::SetSelectedFileSet,
            )
            .into();

            let file_select: Element<ReleaseViewWidgetMessage> =
                if let Some(selected_file_set) = &self.selected_file_set {
                    pick_list(
                        selected_file_set.files.as_slice(),
                        self.selected_file.clone(),
                        ReleaseViewWidgetMessage::SetSelectedFile,
                    )
                    .into()
                } else {
                    text("No file set selected").into()
                };

            column![
                release_name_field,
                software_titles_field,
                system_names_field,
                edit_button,
                file_sets_select,
                file_select,
            ]
            .into()
        } else {
            text("No release selected").into()
        }
    }
}
