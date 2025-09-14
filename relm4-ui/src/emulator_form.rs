use std::sync::Arc;

use core_types::ArgumentType;
use database::{database_error::DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, CheckButtonExt, EditableExt, EntryBufferExtManual, EntryExt,
            GtkWindowExt, OrientableExt, WidgetExt,
        },
    },
};
use service::{
    view_model_service::ViewModelService,
    view_models::{EmulatorListModel, EmulatorViewModel, SystemListModel},
};

use crate::{
    argument_list::{ArgumentList, ArgumentListMsg, ArgumentListOutputMsg},
    system_selector::{
        SystemSelectInit, SystemSelectModel, SystemSelectMsg, SystemSelectOutputMsg,
    },
};

#[derive(Debug)]
pub enum EmulatorFormMsg {
    ExecutableChanged(String),
    NameChanged(String),
    ExtractFilesToggled,
    UpdateExtractFiles(bool),
    SystemSelected(SystemListModel),
    OpenSystemSelector,
    Submit,
    Show {
        editable_emulator: Option<EmulatorViewModel>,
    },
    Hide,
    ArgumentsChanged(Vec<ArgumentType>),
}

#[derive(Debug)]
pub enum EmulatorFormOutputMsg {
    EmulatorAdded(EmulatorListModel),
    EmulatorUpdated(EmulatorListModel),
}

#[derive(Debug)]
pub enum EmulatorFormCommandMsg {
    EmulatorSubmitted(Result<i64, DatabaseError>),
    EmulatorUpdated(Result<i64, DatabaseError>),
}

#[derive(Debug)]
pub struct EmulatorFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub struct EmulatorFormModel {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub selected_system: Option<SystemListModel>,
    system_selector: Controller<SystemSelectModel>,
    editable_emulator_id: Option<i64>,
    argument_list: Controller<ArgumentList>,
    arguments: Vec<ArgumentType>,
}

#[relm4::component(pub)]
impl Component for EmulatorFormModel {
    type Input = EmulatorFormMsg;
    type Output = EmulatorFormOutputMsg;
    type CommandOutput = EmulatorFormCommandMsg;
    type Init = EmulatorFormInit;

    view! {
        gtk::Window {
             connect_close_request[sender] => move |_| {
                 println!("Close request received");
                sender.input(EmulatorFormMsg::Hide);
                glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_top: 10,
                set_margin_bottom: 10,
                set_margin_start: 10,
                set_margin_end: 10,

                gtk::Label {
                    set_label: "Name",
                },

                #[name = "name_entry"]
                gtk::Entry {
                    set_text: &model.name,
                    set_placeholder_text: Some("Emulator name"),
                    connect_changed[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            EmulatorFormMsg::NameChanged(buffer.text().into()),
                        );
                    },
                },

                gtk::Label {
                    set_label: "Executable",
                },

                #[name = "executable_entry"]
                gtk::Entry {
                    set_text: &model.executable,
                    set_placeholder_text: Some("Emulator executable"),
                    connect_changed[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            EmulatorFormMsg::ExecutableChanged(buffer.text().into()),
                        );
                    },
                },

                gtk::CheckButton {
                    set_label: Some("Extract files"),
                    #[watch]
                    #[block_signal(extract_files_toggled)]
                    set_active: model.extract_files,
                    connect_toggled[sender] => move |_| {
                        sender.input(EmulatorFormMsg::ExtractFilesToggled);
                    } @extract_files_toggled,
               },

                gtk::Label {
                    set_label: "Select system:",
                },

                gtk::Label {
                    #[watch]
                    set_label: model.selected_system.as_ref()
                        .map_or("No system selected", |s| s.name.as_str()),
                },

                gtk::Button {
                    set_label: "Select System",
                    connect_clicked => EmulatorFormMsg::OpenSystemSelector,
                },

                gtk::Box {
                    append = model.argument_list.widget(),
                },

                #[name="submit_button"]
                gtk::Button {
                    set_label: "Submit",
                    #[watch]
                    set_sensitive: !model.executable.is_empty() && model.selected_system.is_some(),
                    connect_clicked => EmulatorFormMsg::Submit,
                }
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            EmulatorFormMsg::ExecutableChanged(executable) => {
                println!("Executable changed: {}", executable);
                self.executable = executable;
            }
            EmulatorFormMsg::ExtractFilesToggled => {
                self.extract_files = !self.extract_files;
            }
            EmulatorFormMsg::SystemSelected(system) => {
                println!("System selected: {}", system.name);
                self.selected_system = Some(system);
            }
            EmulatorFormMsg::OpenSystemSelector => {
                let selected_system_ids = self
                    .selected_system
                    .as_ref()
                    .map(|s| s.id)
                    .into_iter()
                    .collect::<Vec<_>>();

                self.system_selector.emit(SystemSelectMsg::Show {
                    selected_system_ids,
                });
            }
            EmulatorFormMsg::Submit => {
                if let Some(system) = &self.selected_system {
                    println!(
                        "Submitting Emulator: {}, Extract Files: {}, System: {:?}",
                        self.executable, self.extract_files, self.selected_system
                    );
                    let repository_manager = Arc::clone(&self.repository_manager);
                    let executable = self.executable.clone();
                    let name = self.name.clone();
                    let extract_files = self.extract_files;
                    let system_id = system.id;

                    let arguments = self.arguments.clone();

                    if let Some(editable_emulator_id) = self.editable_emulator_id {
                        // Update existing emulator
                        sender.oneshot_command(async move {
                            let res = repository_manager
                                .get_emulator_repository()
                                .update_emulator(
                                    editable_emulator_id,
                                    &name,
                                    &executable,
                                    extract_files,
                                    &arguments,
                                    system_id,
                                )
                                .await;
                            EmulatorFormCommandMsg::EmulatorUpdated(res)
                        });
                    } else {
                        sender.oneshot_command(async move {
                            let res = repository_manager
                                .get_emulator_repository()
                                .add_emulator(
                                    &name,
                                    &executable,
                                    extract_files,
                                    &arguments,
                                    system_id,
                                )
                                .await;
                            EmulatorFormCommandMsg::EmulatorSubmitted(res)
                        });
                    }
                }
            }
            EmulatorFormMsg::NameChanged(name) => {
                self.name = name;
            }
            EmulatorFormMsg::UpdateExtractFiles(value) => {
                self.extract_files = value;
            }
            EmulatorFormMsg::Show { editable_emulator } => {
                if let Some(editable_emulator) = editable_emulator {
                    println!("Editing emulator: {:?}", editable_emulator);
                    self.editable_emulator_id = Some(editable_emulator.id);

                    sender.input(EmulatorFormMsg::NameChanged(editable_emulator.name.clone()));
                    sender.input(EmulatorFormMsg::ExecutableChanged(
                        editable_emulator.executable.clone(),
                    ));

                    sender.input(EmulatorFormMsg::UpdateExtractFiles(
                        editable_emulator.extract_files,
                    ));

                    sender.input(EmulatorFormMsg::SystemSelected(
                        editable_emulator.system.clone(),
                    ));

                    self.name = editable_emulator.name.clone();
                    self.executable = editable_emulator.executable.clone();
                    self.extract_files = editable_emulator.extract_files;
                    self.selected_system = Some(editable_emulator.system.clone());

                    println!("Selected system: {:?}", self.selected_system);
                    println!("Executable: {}", self.executable);

                    widgets.name_entry.set_text(&self.name);
                    widgets.executable_entry.set_text(&self.executable);

                    self.argument_list.emit(ArgumentListMsg::SetArguments(
                        editable_emulator.arguments.clone(),
                    ));
                } else {
                    self.editable_emulator_id = None;
                    self.name.clear();
                    self.executable.clear();
                    widgets.name_entry.set_text("");
                    widgets.executable_entry.set_text("");
                    self.extract_files = false;
                    self.selected_system = None;
                    self.argument_list
                        .emit(ArgumentListMsg::SetArguments(Vec::new()));
                }
                root.show();
            }
            EmulatorFormMsg::Hide => {
                println!("Hiding emulator form");
                root.hide();
            }
            EmulatorFormMsg::ArgumentsChanged(arguments) => {
                self.arguments = arguments;
            }
            _ => {}
        }
        // This is essential:
        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            EmulatorFormCommandMsg::EmulatorSubmitted(Ok(id)) => {
                println!("Emulator submitted with id {}", id);
                let name = self.name.clone();
                let res = sender.output(EmulatorFormOutputMsg::EmulatorAdded(EmulatorListModel {
                    id,
                    name,
                }));

                match res {
                    Ok(()) => {
                        root.close();
                    }
                    Err(error) => {
                        eprintln!("Sending message failed: {:?}", error);
                    }
                }
            }
            EmulatorFormCommandMsg::EmulatorUpdated(Ok(id)) => {
                println!("Emulator updated with id {}", id);
                let name = self.name.clone();
                let res =
                    sender.output(EmulatorFormOutputMsg::EmulatorUpdated(EmulatorListModel {
                        id,
                        name,
                    }));

                match res {
                    Ok(()) => {
                        root.close();
                    }
                    Err(error) => {
                        eprintln!("Sending message failed: {:?}", error);
                    }
                }
            }
            EmulatorFormCommandMsg::EmulatorSubmitted(Err(error)) => {
                eprintln!("Error in submitting emulator: {}", error);
                // TODO: show error to user
            }
            EmulatorFormCommandMsg::EmulatorUpdated(Err(error)) => {
                eprintln!("Error in updating emulator: {}", error);
            }
            _ => {
                // Handle command outputs if necessary
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let init_model = SystemSelectInit {
            view_model_service: Arc::clone(&init.view_model_service),
            repository_manager: Arc::clone(&init.repository_manager),
        };

        let system_selector = SystemSelectModel::builder()
            .transient_for(&root)
            .launch(init_model)
            .forward(sender.input_sender(), |msg| match msg {
                SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                    EmulatorFormMsg::SystemSelected(system_list_model)
                }
            });

        let argument_list =
            ArgumentList::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    ArgumentListOutputMsg::ArgumentsChanged(arguments) => {
                        EmulatorFormMsg::ArgumentsChanged(arguments)
                    }
                    _ => unreachable!(),
                });

        let model = Self {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,
            executable: String::new(),
            extract_files: false,
            selected_system: None,
            system_selector,
            name: String::new(),
            editable_emulator_id: None,
            argument_list,
            arguments: Vec::new(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
