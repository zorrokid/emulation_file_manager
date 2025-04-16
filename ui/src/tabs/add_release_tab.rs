use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{widget::text, Task};
use service::{view_model_service::ViewModelService, view_models::SystemListModel};

pub struct AddReleaseTab {
    repositories: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    systems: Vec<SystemListModel>,
}

#[derive(Debug, Clone)]
pub enum Message {
    RepositoriesTestTaskPerformed(String),
}

impl AddReleaseTab {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<Message>) {
        let repositories_clone = Arc::clone(&repositories);
        let repositories_test_task = Task::perform(
            async move {
                match repositories_clone.settings().get_setting("Test").await {
                    Ok(value) => value,
                    Err(_) => {
                        repositories_clone
                            .settings()
                            .add_setting("Test", "value")
                            .await
                            .unwrap();
                        repositories_clone
                            .settings()
                            .get_setting("Test")
                            .await
                            .unwrap()
                    }
                }
            },
            Message::RepositoriesTestTaskPerformed,
        );

        (
            Self {
                repositories,
                view_model_service,
                systems: vec![],
            },
            repositories_test_task,
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::RepositoriesTestTaskPerformed(message) => {
                println!("{}", message);
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        text("Add Release Tab").into()
    }
}
