use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::Task;
use service::{view_model_service::ViewModelService, view_models::SystemListModel};

use super::systems_widget::{self, SystemsWidget};

pub struct EmulatorSystemsWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    // TODO: move systems_widget to EmulatorSystemsAddWidget
    // - there user selects system and adds system specific arguments
    systems_widget: SystemsWidget,
    selected_system_ids: Vec<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Systems(systems_widget::Message),
}

impl EmulatorSystemsWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let (systems_widget, task) =
            SystemsWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));
        (
            Self {
                repositories,
                view_model_service,
                systems_widget,
                selected_system_ids: vec![],
            },
            task.map(Message::Systems),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Systems(message) => {
                let task = self.systems_widget.update(message);
                task.map(Message::Systems)
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let systems_view = self.systems_widget.view().map(Message::Systems);
        systems_view
    }
}
