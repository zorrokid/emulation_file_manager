use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::Task;
use service::view_model_service::ViewModelService;

use crate::widgets::emulators_widget::{EmulatorsWidget, EmulatorsWidgetMessage};

pub struct EmulatorsTab {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    emulators_widget: EmulatorsWidget,
}

#[derive(Debug, Clone)]
pub enum EmulatorsTabMessage {
    EmulatorsWidget(EmulatorsWidgetMessage),
}

impl EmulatorsTab {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<EmulatorsTabMessage>) {
        let (emulators_widget, task) =
            EmulatorsWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));
        (
            Self {
                repositories,
                view_model_service,
                emulators_widget,
            },
            task.map(EmulatorsTabMessage::EmulatorsWidget),
        )
    }

    pub fn update(&mut self, message: EmulatorsTabMessage) -> Task<EmulatorsTabMessage> {
        match message {
            EmulatorsTabMessage::EmulatorsWidget(message) => {
                let task = self.emulators_widget.update(message);
                task.map(EmulatorsTabMessage::EmulatorsWidget)
            }
        }
    }

    pub fn view(&self) -> iced::Element<EmulatorsTabMessage> {
        let emulators_view = self
            .emulators_widget
            .view()
            .map(EmulatorsTabMessage::EmulatorsWidget);
        emulators_view
    }
}
