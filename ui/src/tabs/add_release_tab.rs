use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::Task;
use service::view_model_service::ViewModelService;

use crate::widgets::{
    system_select_widget,
    systems_widget::{self, SystemsWidget},
};

pub struct AddReleaseTab {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems_widget: SystemsWidget,
    selected_system_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub enum Message {
    SystemsWidget(systems_widget::Message),
}

impl AddReleaseTab {
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
                selected_system_ids: vec![],
                systems_widget,
            },
            task.map(Message::SystemsWidget),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SystemsWidget(message) => {
                if let systems_widget::Message::SystemSelect(message) = &message {
                    if let system_select_widget::Message::SystemSelected(system) = message {
                        println!("AddReleaseTab: System selected: {:?}", system);
                        self.selected_system_ids.push(system.id);
                    }
                }
                self.systems_widget
                    .update(message)
                    .map(Message::SystemsWidget)
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        self.systems_widget.view().map(Message::SystemsWidget)
    }
}
