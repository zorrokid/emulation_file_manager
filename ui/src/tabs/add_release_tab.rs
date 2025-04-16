use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{widget::text, Task};

pub struct AddReleaseTab {
    repositories: Arc<RepositoryManager>,
}

#[derive(Debug, Clone)]
pub enum Message {
    RepositoriesTestTaskPerformed(String),
}

impl AddReleaseTab {
    pub fn new(repositories: Arc<RepositoryManager>) -> (Self, Task<Message>) {
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

        (Self { repositories }, repositories_test_task)
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
