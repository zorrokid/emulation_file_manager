use std::sync::Arc;

use database::database_error::Error as DatabaseError;
use database::repository_manager::RepositoryManager;
use iced::widget::{button, column, row, text, Column};
use iced::Task;
use service::error::Error as ServiceError;
use service::view_model_service::ViewModelService;
use service::view_models::{EmulatorListModel, EmulatorViewModel};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::emulator_add_widget::EmulatorAddWidgetMessage;
use super::emulator_select_widget::EmulatorSelectWidgetMessage;
use super::{
    emulator_add_widget::{self, EmulatorAddWidget},
    emulator_select_widget::{self, EmulatorSelectWidget},
};

pub struct EmulatorsWidget {
    emulators: Vec<EmulatorListModel>,
    selected_emulator: Option<EmulatorViewModel>,
    emulator_add_widget: EmulatorAddWidget,
    emulator_select_widget: EmulatorSelectWidget,
    view_model_service: Arc<ViewModelService>,
    repositories: Arc<RepositoryManager>,
}

#[derive(Debug, Clone)]
pub enum EmulatorsWidgetMessage {
    // child messages
    EmulatorAddWidget(EmulatorAddWidgetMessage),
    EmulatorSelectWidget(EmulatorSelectWidgetMessage),
    // local messages
    EmulatorsFetched(Result<Vec<EmulatorListModel>, ServiceError>),
    SelectedEmulatorLoaded(Result<EmulatorViewModel, ServiceError>),
    RemoveEmulator(i64),
    EditEmulator(i64),
    EmulatorDeleted(Result<i64, DatabaseError>),
}

impl EmulatorsWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<EmulatorsWidgetMessage>) {
        let view_model_service_clone = Arc::clone(&view_model_service);
        let fech_emulators_task = Task::perform(
            async move { view_model_service_clone.get_emulator_list_models().await },
            EmulatorsWidgetMessage::EmulatorsFetched,
        );
        let (emulator_add_widget, emulators_task) =
            EmulatorAddWidget::new(Arc::clone(&repositories), Arc::clone(&view_model_service));

        let combined_task = Task::batch(vec![
            fech_emulators_task,
            emulators_task.map(EmulatorsWidgetMessage::EmulatorAddWidget),
        ]);
        (
            Self {
                emulators: vec![],
                selected_emulator: None,
                emulator_add_widget,
                emulator_select_widget: EmulatorSelectWidget::new(),
                view_model_service,
                repositories,
            },
            combined_task,
        )
    }

    pub fn update(&mut self, message: EmulatorsWidgetMessage) -> Task<EmulatorsWidgetMessage> {
        match message {
            EmulatorsWidgetMessage::EmulatorsFetched(result) => match result {
                Ok(emulators) => {
                    self.emulators = emulators;
                    println!("Emulators fetched: {:?}", self.emulators);
                    // TODO: would it be possible to handle the EmulatosFetched message in the
                    // emulator_select_widget?
                    return self
                        .emulator_select_widget
                        .update(
                            emulator_select_widget::EmulatorSelectWidgetMessage::SetEmulators(
                                self.emulators.clone(),
                            ),
                        )
                        .map(EmulatorsWidgetMessage::EmulatorSelectWidget);
                }
                Err(error) => {
                    eprintln!("Error fetching emulators: {:?}", error);
                }
            },
            EmulatorsWidgetMessage::EmulatorAddWidget(msg) => {
                if let emulator_add_widget::EmulatorAddWidgetMessage::EmulatorAdded(
                    emulator_list_model,
                ) = msg.clone()
                {
                    println!("Emulator added: {:?}", emulator_list_model);
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let fetch_emulator_task = Task::perform(
                        async move {
                            view_model_service
                                .get_emulator_view_model(emulator_list_model.id)
                                .await
                        },
                        EmulatorsWidgetMessage::SelectedEmulatorLoaded,
                    );
                    self.emulators.push(emulator_list_model);
                    return fetch_emulator_task;
                }
                println!("Updating emulator add widget with message: {:?}", msg);
                return self
                    .emulator_add_widget
                    .update(msg)
                    .map(EmulatorsWidgetMessage::EmulatorAddWidget);
            }
            EmulatorsWidgetMessage::EmulatorSelectWidget(msg) => {
                let update_task = self
                    .emulator_select_widget
                    .update(msg.clone())
                    .map(EmulatorsWidgetMessage::EmulatorSelectWidget);
                if let emulator_select_widget::EmulatorSelectWidgetMessage::EmulatorSelected(
                    emulator_list_model,
                ) = &msg
                {
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let id = emulator_list_model.id;
                    let fetch_emulator_task = Task::perform(
                        async move { view_model_service.get_emulator_view_model(id).await },
                        EmulatorsWidgetMessage::SelectedEmulatorLoaded,
                    );
                    return Task::batch(vec![fetch_emulator_task, update_task]);
                }
                return update_task;
            }
            EmulatorsWidgetMessage::SelectedEmulatorLoaded(result) => match result {
                Ok(emulator) => {
                    self.selected_emulator = Some(emulator);
                }
                Err(error) => {
                    eprintln!("Error loading selected emulator: {:?}", error);
                }
            },
            EmulatorsWidgetMessage::RemoveEmulator(id) => {
                let repositories = Arc::clone(&self.repositories);
                let remove_emulator_task = Task::perform(
                    async move {
                        repositories
                            .get_emulator_repository()
                            .delete_emulator(id)
                            .await
                    },
                    EmulatorsWidgetMessage::EmulatorDeleted,
                );
                return remove_emulator_task;
            }
            EmulatorsWidgetMessage::EditEmulator(id) => {
                return self
                    .emulator_add_widget
                    .update(emulator_add_widget::EmulatorAddWidgetMessage::SetEmulatorId(id))
                    .map(EmulatorsWidgetMessage::EmulatorAddWidget);
            }
            EmulatorsWidgetMessage::EmulatorDeleted(result) => match result {
                Ok(id) => {
                    println!("Emulator deleted successfully with id: {:?}", id);
                    self.emulators.retain(|e| e.id != id);
                    self.selected_emulator = None;
                }
                Err(error) => {
                    eprintln!("Error deleting emulator: {:?}", error);
                }
            },
        }
        Task::none()
    }

    pub fn view(&self) -> iced::Element<EmulatorsWidgetMessage> {
        let emulator_add_view = self
            .emulator_add_widget
            .view()
            .map(EmulatorsWidgetMessage::EmulatorAddWidget);

        let emulator_select_view = self
            .emulator_select_widget
            .view()
            .map(EmulatorsWidgetMessage::EmulatorSelectWidget);

        column![
            emulator_select_view,
            self.create_selected_emulator_view(),
            emulator_add_view,
        ]
        .into()
    }

    pub fn create_selected_emulator_view(&self) -> iced::Element<EmulatorsWidgetMessage> {
        let row_content = match &self.selected_emulator {
            Some(emulator) => {
                let emulator_name = text!("{}", emulator.name.clone());
                let remove_button =
                    button("Remove").on_press(EmulatorsWidgetMessage::RemoveEmulator(emulator.id));
                let edit_button =
                    button("Edit").on_press(EmulatorsWidgetMessage::EditEmulator(emulator.id));

                let emulator_name = row![emulator_name, remove_button, edit_button]
                    .spacing(DEFAULT_SPACING)
                    .padding(DEFAULT_PADDING);
                let systems: Vec<iced::Element<EmulatorsWidgetMessage>> = emulator
                    .systems
                    .iter()
                    .map(|system| text!("{}: {}", system.system_name, system.arguments).into())
                    .collect();
                column![emulator_name, Column::with_children(systems)]
            }
            None => column![text("No emulator selected")],
        };
        row![row_content]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .into()
    }
}
