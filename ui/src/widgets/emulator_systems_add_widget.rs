use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use iced::{
    widget::{button, column, container, text, text_input, Column, Container},
    Element, Task,
};
use service::{view_model_service::ViewModelService, view_models::SystemListModel};

use crate::defaults::{DEFAULT_PADDING, DEFAULT_SPACING};

use super::{
    emulator_add_widget::EmulatorSystem,
    system_select_widget,
    systems_widget::{self, SystemWidgetMessage, SystemsWidget},
};

pub struct EmulatorSystemsAddWidget {
    systems_widget: SystemsWidget,
    selected_system: Option<SystemListModel>,
    arguments: String,
    is_open: bool,
    emulator_system_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum EmulatorSystemsAddWidgetMessage {
    // child messages
    SystemsWidget(SystemWidgetMessage),
    // local messages
    ArgumentsChanged(String),
    Submit,
    AddEmulatorSystem(EmulatorSystem),
    ToggleOpen,
    SetEmulatorSystem(EmulatorSystem),
}

impl EmulatorSystemsAddWidget {
    pub fn new(
        repositories: Arc<RepositoryManager>,
        view_model_service: Arc<ViewModelService>,
    ) -> (Self, Task<EmulatorSystemsAddWidgetMessage>) {
        let (systems_widget, task) = SystemsWidget::new(repositories, view_model_service);

        (
            Self {
                systems_widget,
                selected_system: None,
                arguments: String::new(),
                is_open: false,
                emulator_system_id: None,
            },
            task.map(EmulatorSystemsAddWidgetMessage::SystemsWidget),
        )
    }

    pub fn update(
        &mut self,
        message: EmulatorSystemsAddWidgetMessage,
    ) -> Task<EmulatorSystemsAddWidgetMessage> {
        match message {
            EmulatorSystemsAddWidgetMessage::SystemsWidget(message) => {
                if let systems_widget::SystemWidgetMessage::SystemSelect(
                    system_select_widget::SystemSelectWidgetMessage::SystemSelected(system),
                ) = &message
                {
                    self.selected_system = Some(system.clone());
                }
                let task = self.systems_widget.update(message);
                task.map(EmulatorSystemsAddWidgetMessage::SystemsWidget)
            }
            EmulatorSystemsAddWidgetMessage::ArgumentsChanged(arguments) => {
                self.arguments = arguments;
                Task::none()
            }
            EmulatorSystemsAddWidgetMessage::Submit => {
                if let Some(system) = &self.selected_system {
                    let system_id = system.id;
                    let system_name = system.name.clone();
                    let arguments = self.arguments.clone();
                    let id = self.emulator_system_id;
                    self.selected_system = None;
                    self.arguments = String::new();
                    self.is_open = false;
                    return Task::done(EmulatorSystemsAddWidgetMessage::AddEmulatorSystem(
                        EmulatorSystem {
                            id,
                            system_id,
                            system_name,
                            arguments,
                        },
                    ));
                }
                Task::none()
            }
            EmulatorSystemsAddWidgetMessage::ToggleOpen => {
                self.is_open = !self.is_open;
                Task::none()
            }
            EmulatorSystemsAddWidgetMessage::SetEmulatorSystem(emulator_system) => {
                self.emulator_system_id = emulator_system.id;
                self.arguments = emulator_system.arguments.clone();
                self.selected_system = Some(SystemListModel {
                    id: emulator_system.system_id,
                    name: emulator_system.system_name.clone(),
                    can_delete: false,
                });
                self.is_open = true;
                Task::none()
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> Element<EmulatorSystemsAddWidgetMessage> {
        let emulator_system_add_or_edit_view = if self.is_open {
            self.create_add_or_edit_emulator_system_view()
        } else {
            Column::new().push(
                button("Add Emulator System").on_press(EmulatorSystemsAddWidgetMessage::ToggleOpen),
            )
        };
        Container::new(
            emulator_system_add_or_edit_view
                .spacing(DEFAULT_SPACING)
                .padding(DEFAULT_PADDING),
        )
        .style(container::bordered_box)
        .into()
    }

    fn create_add_or_edit_emulator_system_view(&self) -> Column<EmulatorSystemsAddWidgetMessage> {
        let cancel_button_text = if self.emulator_system_id.is_some() {
            "Cancel edit emulator system"
        } else {
            "Cancel add emulator system"
        };
        let cancel_add_emulator_system_button =
            button(cancel_button_text).on_press(EmulatorSystemsAddWidgetMessage::ToggleOpen);

        let systems_view = self
            .systems_widget
            .view()
            .map(EmulatorSystemsAddWidgetMessage::SystemsWidget);
        let selected_system_name = self.selected_system.as_ref().map_or("None", |s| &s.name);
        let selected_system_text = text!("Selected System: {}", &selected_system_name);
        let add_argument_input = text_input("Add system specific arguments", &self.arguments)
            .on_input(EmulatorSystemsAddWidgetMessage::ArgumentsChanged);
        let submit_button = button("Submit").on_press(EmulatorSystemsAddWidgetMessage::Submit);
        column![
            cancel_add_emulator_system_button,
            selected_system_text,
            add_argument_input,
            systems_view,
            submit_button
        ]
        .spacing(DEFAULT_SPACING)
        .padding(DEFAULT_PADDING)
    }
}
