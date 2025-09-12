use std::sync::Arc;

use core_types::{ArgumentType, DocumentType};
use database::{database_error::DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
};
use service::{view_model_service::ViewModelService, view_models::DocumentViewerListModel};

use crate::argument_list::{ArgumentList, ArgumentListOutputMsg};

#[derive(Debug)]
pub enum DocumentViewerFormMsg {
    ExecutableChanged(String),
    NameChanged(String),
    Submit,
    Show,
    Hide,
    ArgumentsChanged(Vec<ArgumentType>),
}

#[derive(Debug)]
pub enum DocumentViewerFormOutputMsg {
    DocumentViewerAdded(DocumentViewerListModel),
}

#[derive(Debug)]
pub enum DocumentViewerFormCommandMsg {
    DocumentViewerSubmitted(Result<i64, DatabaseError>),
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
    pub selected_document_type: Option<DocumentType>,
    argument_list: Controller<ArgumentList>,
    arguments: Vec<ArgumentType>,
}

#[relm4::component(pub)]
impl Component for DocumentViewerFormModel {
    type Input = DocumentViewerFormMsg;
    type Output = DocumentViewerFormOutputMsg;
    type CommandOutput = DocumentViewerFormCommandMsg;
    type Init = DocumentViewerFormInit;

    view! {
        gtk::Window {
            set_title: Some("Document viewer form"),

            connect_close_request[sender] => move |_| {
                sender.input(DocumentViewerFormMsg::Hide);
                glib::Propagation::Proceed
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

                gtk::Box {
                    append = model.argument_list.widget(),
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
            DocumentViewerFormMsg::Submit => {
                if let Some(document_type) = self.selected_document_type {
                    let repository_manager = Arc::clone(&self.repository_manager);
                    let executable = self.executable.clone();
                    let name = self.name.clone();
                    let arguments = self.arguments.clone();

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
            DocumentViewerFormMsg::Show => {
                root.show();
            }
            DocumentViewerFormMsg::Hide => {
                root.hide();
            }
            DocumentViewerFormMsg::ArgumentsChanged(arguments) => {
                self.arguments = arguments;
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
        let argument_list =
            ArgumentList::builder()
                .launch(())
                .forward(sender.input_sender(), |msg| match msg {
                    ArgumentListOutputMsg::ArgumentsChanged(arguments) => {
                        DocumentViewerFormMsg::ArgumentsChanged(arguments)
                    }
                    _ => unreachable!(),
                });

        let model = Self {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,
            executable: String::new(),
            arguments: Vec::new(),
            name: String::new(),
            // TODO: add document type selection
            selected_document_type: Some(DocumentType::Pdf),
            argument_list,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
