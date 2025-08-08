use std::sync::Arc;

use database::{
    database_error::{DatabaseError, Error},
    models::Emulator,
    repository_manager::RepositoryManager,
};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, FactorySender,
    gtk::{
        self,
        glib::clone,
        prelude::{
            ButtonExt, CheckButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
    prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque},
    typed_view::list::TypedListView,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{EmulatorListModel, EmulatorViewModel, SystemListModel},
};

use crate::{
    list_item::ListItem,
    system_selector::{SystemSelectInit, SystemSelectModel, SystemSelectOutputMsg},
};

#[derive(Debug)]
pub struct CommandLineArgument {
    value: String,
}

#[derive(Debug)]
pub enum CommandLineArgumentInput {}

#[derive(Debug)]
pub enum CommandLineArgumentOutput {
    Delete(DynamicIndex),
}

#[relm4::factory(pub)]
impl FactoryComponent for CommandLineArgument {
    type Init = String;
    type Input = CommandLineArgumentInput;
    type Output = CommandLineArgumentOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            #[name(label)]
            gtk::Label {
                set_label: &self.value,
            },
            gtk::Button {
               set_icon_name: "edit-delete",
                connect_clicked[sender, index] => move |_| {
                    let res = sender.output(CommandLineArgumentOutput::Delete(index.clone()));
                    match res {
                        Ok(()) => {
                            println!("Command line argument deleted: {:?}", index);
                        }
                        Err(error) => {
                            eprintln!("Error sending delete message: {:?}", error);
                        }
                    }
                },
            }
        }
    }

    fn init_model(value: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { value }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {}
    }
}

#[derive(Debug)]
pub enum EmulatorFormMsg {
    ExecutableChanged(String),
    NameChanged(String),
    ExtractFilesToggled,
    SystemSelected(SystemListModel),
    SystemFocused { index: u32 },
    OpenSystemSelector,
    AddCommandLineArgument(String),
    DeleteCommandLineArgument(DynamicIndex),
    Submit,
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
    pub editable_emulator: Option<EmulatorViewModel>,
}

#[derive(Debug)]
pub struct EmulatorFormModel {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub selected_system: Option<SystemListModel>,
    system_selector: Option<Controller<SystemSelectModel>>,
    pub command_line_arguments: FactoryVecDeque<CommandLineArgument>,
    pub arguments: Vec<String>,
    editable_emulator_id: Option<i64>,
}

#[relm4::component(pub)]
impl Component for EmulatorFormModel {
    type Input = EmulatorFormMsg;
    type Output = EmulatorFormOutputMsg;
    type CommandOutput = EmulatorFormCommandMsg;
    type Init = EmulatorFormInit;

    view! {
        gtk::Window {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_top: 10,
                set_margin_bottom: 10,
                set_margin_start: 10,
                set_margin_end: 10,

                gtk::Label {
                    set_label: "Name",
                },

                gtk::Entry {
                    set_text: &model.name,
                    set_placeholder_text: Some("Emulator name"),
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            EmulatorFormMsg::NameChanged(buffer.text().into()),
                        );
                    }
                },

                gtk::Label {
                    set_label: "Executable",
                },
                gtk::Entry {
                    set_text: &model.executable,
                    set_placeholder_text: Some("Emulator executable"),
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            EmulatorFormMsg::ExecutableChanged(buffer.text().into()),
                        );
                    },
                },

                gtk::CheckButton {
                    set_label: Some("Extract files"),
                    set_active: model.extract_files,
                    connect_toggled => EmulatorFormMsg::ExtractFilesToggled
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

                gtk::Label {
                    set_label: "Add command line argument",
                },

                gtk::Entry {
                    #[watch]
                    set_sensitive: model.selected_system.is_some(),
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(EmulatorFormMsg::AddCommandLineArgument(buffer.text().into()));
                        buffer.delete_text(0, None);
                    }
                },

                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_min_content_height: 360,
                    set_vexpand: true,

                    #[local_ref]
                    command_line_argument_list_box -> gtk::ListBox {}
                },

                gtk::Button {
                    set_label: "Submit",
                    #[watch]
                    set_sensitive: !model.executable.is_empty() && model.selected_system.is_some(),
                    connect_clicked => EmulatorFormMsg::Submit,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EmulatorFormMsg::ExecutableChanged(executable) => {
                println!("Executable changed: {}", executable);
                self.executable = executable;
            }
            EmulatorFormMsg::ExtractFilesToggled => {
                self.extract_files = !self.extract_files;
                println!("Extract files toggled: {}", self.extract_files);
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
                let init_model = SystemSelectInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                    selected_system_ids,
                };
                let system_selector = SystemSelectModel::builder()
                    .transient_for(root)
                    .launch(init_model)
                    .forward(sender.input_sender(), |msg| match msg {
                        SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                            EmulatorFormMsg::SystemSelected(system_list_model)
                        }
                    });
                self.system_selector = Some(system_selector);

                self.system_selector
                    .as_ref()
                    .expect("System selector should be set")
                    .widget()
                    .present();
            }
            EmulatorFormMsg::AddCommandLineArgument(argument) => {
                self.command_line_arguments
                    .guard()
                    .push_back(argument.clone());
                self.arguments.push(argument);
            }
            EmulatorFormMsg::DeleteCommandLineArgument(index) => {
                self.command_line_arguments
                    .guard()
                    .remove(index.current_index());
                self.arguments.remove(index.current_index());
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
                    let arguments = self.arguments.join("|");
                    if let Some(editable_emulator_id) = self.editable_emulator_id {
                        // Update existing emulator
                        sender.oneshot_command(async move {
                            let update_emulator = Emulator {
                                id: editable_emulator_id,
                                name,
                                executable,
                                extract_files,
                                arguments,
                                system_id,
                            };
                            let res = repository_manager
                                .get_emulator_repository()
                                .update_emulator(&update_emulator)
                                .await;
                            EmulatorFormCommandMsg::EmulatorUpdated(res)
                        });
                    } else {
                        sender.oneshot_command(async move {
                            let res = repository_manager
                                .get_emulator_repository()
                                .add_emulator(name, executable, extract_files, arguments, system_id)
                                .await;
                            EmulatorFormCommandMsg::EmulatorSubmitted(res)
                        });
                    }
                }
            }
            EmulatorFormMsg::NameChanged(name) => {
                self.name = name;
            }
            _ => {}
        }
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
        let mut command_line_arguments =
            FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), |msg| match msg {
                    CommandLineArgumentOutput::Delete(index) => {
                        EmulatorFormMsg::DeleteCommandLineArgument(index)
                    }
                });

        let model = match init.editable_emulator {
            Some(editable_emulator) => {
                editable_emulator.arguments.iter().for_each(|arg| {
                    command_line_arguments.guard().push_back(arg.clone());
                });

                Self {
                    view_model_service: init.view_model_service,
                    repository_manager: init.repository_manager,
                    executable: editable_emulator.executable.clone(),
                    extract_files: editable_emulator.extract_files,
                    selected_system: Some(editable_emulator.system.clone()),
                    system_selector: None,
                    command_line_arguments,
                    arguments: editable_emulator.arguments.clone(),
                    name: editable_emulator.name.clone(),
                    editable_emulator_id: Some(editable_emulator.id),
                }
            }
            None => Self {
                view_model_service: init.view_model_service,
                repository_manager: init.repository_manager,
                executable: String::new(),
                extract_files: false,
                selected_system: None,
                system_selector: None,
                command_line_arguments,
                arguments: Vec::new(),
                name: String::new(),
                editable_emulator_id: None,
            },
        };

        let command_line_argument_list_box = model.command_line_arguments.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
