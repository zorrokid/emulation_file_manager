use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{Element, Task};
use service::{view_model_service::ViewModelService, view_models::SystemListModel};

use super::systems_widget::{SystemWidgetMessage, SystemsWidget};

pub struct EmulatorSystemsWidget {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    // TODO: move systems_widget to EmulatorSystemsAddWidget
    // - there user selects system and adds system specific arguments
    systems_widget: SystemsWidget,
    selected_system_ids: Vec<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum EmulatorSystemsWidgetMessage {
    SystemsWidget(SystemWidgetMessage),
}

impl EmulatorSystemsWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<EmulatorSystemsWidgetMessage>) {
        let (systems_widget, task) =
            SystemsWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));
        (
            Self {
                repositories,
                view_model_service,
                systems_widget,
                selected_system_ids: vec![],
            },
            task.map(EmulatorSystemsWidgetMessage::SystemsWidget),
        )
    }

    pub fn update(
        &mut self,
        message: EmulatorSystemsWidgetMessage,
    ) -> Task<EmulatorSystemsWidgetMessage> {
        match message {
            EmulatorSystemsWidgetMessage::SystemsWidget(message) => {
                let task = self.systems_widget.update(message);
                task.map(EmulatorSystemsWidgetMessage::SystemsWidget)
            }
        }
    }

    pub fn view(&self) -> Element<EmulatorSystemsWidgetMessage> {
        let systems_view = self
            .systems_widget
            .view()
            .map(EmulatorSystemsWidgetMessage::SystemsWidget);
        systems_view
    }
}
