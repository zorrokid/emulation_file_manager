use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, FactorySender,
    gtk::{
        self,
        prelude::{
            ButtonExt, CheckButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
    prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque},
    typed_view::list::TypedListView,
};
use service::{view_model_service::ViewModelService, view_models::SystemListModel};

use crate::{
    list_item::ListItem,
    system_selector::{SystemSelectInit, SystemSelectModel, SystemSelectOutputMsg},
};

#[derive(Debug)]
struct CommandLineArgument {
    value: String,
}

#[derive(Debug)]
enum CommandLineArgumentInput {}

#[derive(Debug)]
enum CommandLineArgumentOutput {
    Delete(DynamicIndex),
}

#[relm4::factory]
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
                    sender.output(CommandLineArgumentOutput::Delete(index.clone()));
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
    ExtractFilesToggled,
    SystemSelected(SystemListModel),
    OpenSystemSelector,
    AddCommandLineArgument(String),
    DeleteCommandLineArgument(DynamicIndex),
    Submit,
}

#[derive(Debug)]
pub enum EmulatorFormOutputMsg {}

#[derive(Debug)]
pub enum EmulatorFormCommandMsg {}

pub struct EmulatorFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub struct EmulatorFormModel {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub executable: String,
    pub extract_files: bool,
    pub selected_systems: Vec<SystemListModel>,
    pub selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    system_selector: Option<Controller<SystemSelectModel>>,
    pub command_line_arguments: FactoryVecDeque<CommandLineArgument>,
}

#[relm4::component(pub)]
impl Component for EmulatorFormModel {
    type Input = EmulatorFormMsg;
    type Output = EmulatorFormOutputMsg;
    type CommandOutput = EmulatorFormCommandMsg;
    type Init = EmulatorFormInit;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_top: 10,
            set_margin_bottom: 10,
            set_margin_start: 10,
            set_margin_end: 10,

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
                    buffer.delete_text(0, None);
                },
            },

            gtk::CheckButton {
                set_label: Some("Extract files"),
                set_active: model.extract_files,
                connect_toggled => EmulatorFormMsg::ExtractFilesToggled
            },

            #[local_ref]
            selected_systems_list_view -> gtk::ListView { },

            gtk::Button {
                set_label: "Select System",
                connect_clicked => EmulatorFormMsg::OpenSystemSelector,
            },

            gtk::Entry {
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
                set_sensitive: !model.executable.is_empty() && !model.selected_systems.is_empty(),
                connect_clicked => EmulatorFormMsg::Submit,
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            EmulatorFormMsg::ExecutableChanged(executable) => {
                self.executable = executable;
            }
            EmulatorFormMsg::ExtractFilesToggled => {
                self.extract_files = !self.extract_files;
            }
            EmulatorFormMsg::SystemSelected(system) => {
                self.selected_systems_list_view_wrapper.append(ListItem {
                    name: system.name.clone(),
                    id: system.id,
                });
                self.selected_systems.push(system);
            }
            EmulatorFormMsg::OpenSystemSelector => {
                let init_model = SystemSelectInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
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
        let selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let command_line_arguments =
            FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), |msg| match msg {
                    CommandLineArgumentOutput::Delete(index) => {
                        EmulatorFormMsg::DeleteCommandLineArgument(index)
                    }
                });

        let model = Self {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,
            executable: String::new(),
            extract_files: false,
            selected_systems: Vec::new(),
            selected_systems_list_view_wrapper,
            system_selector: None,
            command_line_arguments,
        };

        let selected_systems_list_view = &model.selected_systems_list_view_wrapper.view;
        let command_line_argument_list_box = model.command_line_arguments.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
