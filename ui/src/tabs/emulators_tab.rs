use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::Task;
use service::view_model_service::ViewModelService;

use crate::widgets::emulators_widget::{self, EmulatorsWidget};

pub struct EmulatorsTab {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    emulators_widget: EmulatorsWidget,
}

#[derive(Debug, Clone)]
pub enum Message {
    Emulators(emulators_widget::Message),
}

impl EmulatorsTab {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let (emulators_widget, task) =
            EmulatorsWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));
        (
            Self {
                repositories,
                view_model_service,
                emulators_widget,
            },
            task.map(Message::Emulators),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Emulators(message) => {
                let task = self.emulators_widget.update(message);
                task.map(Message::Emulators)
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let emulators_view = self.emulators_widget.view().map(Message::Emulators);
        emulators_view
    }
}
