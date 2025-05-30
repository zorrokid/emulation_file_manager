use std::sync::Arc;

use iced::{
    widget::{button, column, text},
    Element, Task,
};
use service::{view_model_service::ViewModelService, view_models::ReleaseViewModel};

use super::emulator_runner_widget::{EmulatorRunnerWidget, EmulatorRunnerWidgetMessage};

pub struct ReleaseViewWidget {
    release: Option<ReleaseViewModel>,
    view_model_service: Arc<ViewModelService>,
    emulator_runner_widget: EmulatorRunnerWidget,
}

#[derive(Debug, Clone)]
pub enum ReleaseViewWidgetMessage {
    EmulatorRunnerWidget(EmulatorRunnerWidgetMessage),
    SetEditRelease(ReleaseViewModel),
    SetRelease(ReleaseViewModel),
    // Local messages
    StartEditRelease,
}

impl ReleaseViewWidget {
    pub fn new(
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<ReleaseViewWidgetMessage>) {
        let (emulator_runner_widget, emulator_runner_task) =
            EmulatorRunnerWidget::new(Arc::clone(&view_model_service));

        (
            ReleaseViewWidget {
                release: None,
                view_model_service,
                emulator_runner_widget,
            },
            emulator_runner_task.map(ReleaseViewWidgetMessage::EmulatorRunnerWidget),
        )
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
                self.release = Some(release.clone());
                self.emulator_runner_widget
                    .update(EmulatorRunnerWidgetMessage::ReleaseChanged(release))
                    .map(ReleaseViewWidgetMessage::EmulatorRunnerWidget)
                // TODO: reset emulator runner widget state
            }
            ReleaseViewWidgetMessage::EmulatorRunnerWidget(emulator_runner_message) => self
                .emulator_runner_widget
                .update(emulator_runner_message)
                .map(ReleaseViewWidgetMessage::EmulatorRunnerWidget),
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<ReleaseViewWidgetMessage> {
        if let Some(release) = &self.release {
            let release_name_field = text!("Release Name: {}", release.name);
            let software_titles_field = text!("Software Titles: {:?}", release.software_titles);
            let system_names_field = text!("Systems: {:?}", release.systems);
            let edit_button = button("Edit").on_press(ReleaseViewWidgetMessage::StartEditRelease);

            let emulator_runner_view = self
                .emulator_runner_widget
                .view()
                .map(ReleaseViewWidgetMessage::EmulatorRunnerWidget);

            column![
                release_name_field,
                software_titles_field,
                system_names_field,
                edit_button,
                emulator_runner_view,
            ]
            .into()
        } else {
            text("No release selected").into()
        }
    }
}
