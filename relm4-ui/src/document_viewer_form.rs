use std::sync::Arc;

use core_types::DocumentType;
use database::{database_error::Error, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        prelude::{
            ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt, OrientableExt,
            WidgetExt,
        },
    },
    prelude::{DynamicIndex, FactoryVecDeque},
};
use service::{view_model_service::ViewModelService, view_models::DocumentViewerListModel};

use crate::emulator_form::{CommandLineArgument, CommandLineArgumentOutput};

#[derive(Debug)]
pub enum DocumentViewerFormMsg {
    ExecutableChanged(String),
    NameChanged(String),
    ExtractFilesToggled,
    AddCommandLineArgument(String),
    DeleteCommandLineArgument(DynamicIndex),
    Submit,
}

#[derive(Debug)]
pub enum DocumentViewerFormOutputMsg {
    DocumentViewerAdded(DocumentViewerListModel),
}

#[derive(Debug)]
pub enum DocumentViewerFormCommandMsg {
    DocumentViewerSubmitted(Result<i64, Error>),
}

pub struct DocumentViewerFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub struct DocumentViewerFormModel {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub name: String,
    pub executable: String,
    pub extract_files: bool,
    pub command_line_arguments: FactoryVecDeque<CommandLineArgument>,
    pub arguments: Vec<String>,
    pub selected_document_type: Option<DocumentType>,
}

#[relm4::component(pub)]
impl Component for DocumentViewerFormModel {
    type Input = DocumentViewerFormMsg;
    type Output = DocumentViewerFormOutputMsg;
    type CommandOutput = DocumentViewerFormCommandMsg;
    type Init = DocumentViewerFormInit;

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
                    set_placeholder_text: Some("DocumentViewer name"),
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            DocumentViewerFormMsg::NameChanged(buffer.text().into()),
                        );
                    }
                },

                gtk::Label {
                    set_label: "Executable",
                },
                gtk::Entry {
                    set_text: &model.executable,
                    set_placeholder_text: Some("DocumentViewer executable"),
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            DocumentViewerFormMsg::ExecutableChanged(buffer.text().into()),
                        );
                    },
                },

                gtk::Label {
                    set_label: "Add command line argument",
                },

                gtk::Entry {
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(DocumentViewerFormMsg::AddCommandLineArgument(buffer.text().into()));
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
                    set_sensitive: !model.executable.is_empty(),
                    connect_clicked => DocumentViewerFormMsg::Submit,
                }
            }
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            DocumentViewerFormMsg::ExecutableChanged(executable) => {
                println!("Executable changed: {}", executable);
                self.executable = executable;
            }
            DocumentViewerFormMsg::AddCommandLineArgument(argument) => {
                self.command_line_arguments
                    .guard()
                    .push_back(argument.clone());
                self.arguments.push(argument);
            }
            DocumentViewerFormMsg::DeleteCommandLineArgument(index) => {
                self.command_line_arguments
                    .guard()
                    .remove(index.current_index());
            }

            DocumentViewerFormMsg::Submit => {
                if let Some(document_type) = self.selected_document_type {
                    let repository_manager = Arc::clone(&self.repository_manager);
                    let executable = self.executable.clone();
                    let name = self.name.clone();
                    let arguments = self.arguments.join("|");

                    sender.oneshot_command(async move {
                        let res = repository_manager
                            .get_document_viewer_repository()
                            .add_document_viewer(&name, &executable, &arguments, &document_type)
                            .await;
                        DocumentViewerFormCommandMsg::DocumentViewerSubmitted(res)
                    });
                }
            }
            DocumentViewerFormMsg::NameChanged(name) => {
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
            DocumentViewerFormCommandMsg::DocumentViewerSubmitted(Ok(id)) => {
                println!("DocumentViewer submitted with id {}", id);
                let name = self.name.clone();
                let res = sender.output(DocumentViewerFormOutputMsg::DocumentViewerAdded(
                    DocumentViewerListModel { id, name },
                ));

                match res {
                    Ok(()) => {
                        root.close();
                    }
                    Err(error) => {
                        eprintln!("Sending message failed: {:?}", error);
                    }
                }
            }
            DocumentViewerFormCommandMsg::DocumentViewerSubmitted(Err(error)) => {
                eprintln!("Error in submitting document_viewer: {}", error);
                // TODO: show error to user
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
        let command_line_arguments =
            FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), |msg| match msg {
                    CommandLineArgumentOutput::Delete(index) => {
                        DocumentViewerFormMsg::DeleteCommandLineArgument(index)
                    }
                });

        let model = Self {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,
            executable: String::new(),
            extract_files: false,
            command_line_arguments,
            arguments: Vec::new(),
            name: String::new(),
            // TODO: add document type selection
            selected_document_type: Some(DocumentType::Pdf),
        };

        let command_line_argument_list_box = model.command_line_arguments.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
