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
use service::{
    view_model_service::ViewModelService,
    view_models::{DocumentViewerListModel, DocumentViewerViewModel},
};

use crate::argument_list::{ArgumentList, ArgumentListMsg, ArgumentListOutputMsg};

#[derive(Debug)]
pub enum DocumentViewerFormMsg {
    ExecutableChanged(String),
    NameChanged(String),
    Submit,
    Show {
        edit_document_viewer: Option<DocumentViewerViewModel>,
    },
    Hide,
    ArgumentsChanged(Vec<ArgumentType>),
}

#[derive(Debug)]
pub enum DocumentViewerFormOutputMsg {
    DocumentViewerAdded(DocumentViewerListModel),
    DocumentViewerUpdated(DocumentViewerListModel),
}

#[derive(Debug)]
pub enum DocumentViewerFormCommandMsg {
    DocumentViewerSubmitted(Result<i64, DatabaseError>),
    DocumentViewerUpdated(Result<i64, DatabaseError>),
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
    editable_viewer_id: Option<i64>,
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

                #[name = "executable_entry"]
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

                // TODO: add file type selection

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

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
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

                    if let Some(editable_id) = self.editable_viewer_id {
                        sender.oneshot_command(async move {
                            let res = repository_manager
                                .get_document_viewer_repository()
                                .update_document_viewer(
                                    editable_id,
                                    &name,
                                    &executable,
                                    &arguments,
                                    &document_type,
                                )
                                .await;
                            DocumentViewerFormCommandMsg::DocumentViewerUpdated(res)
                        });
                    } else {
                        sender.oneshot_command(async move {
                            let res = repository_manager
                                .get_document_viewer_repository()
                                .add_document_viewer(&name, &executable, &arguments, &document_type)
                                .await;
                            DocumentViewerFormCommandMsg::DocumentViewerSubmitted(res)
                        });
                    }
                }
            }
            DocumentViewerFormMsg::NameChanged(name) => {
                self.name = name;
            }
            DocumentViewerFormMsg::Show {
                edit_document_viewer,
            } => {
                if let Some(editable_viewer) = edit_document_viewer {
                    println!("Editing document viewer: {:?}", editable_viewer);
                    self.editable_viewer_id = Some(editable_viewer.id);
                    self.name = editable_viewer.name;
                    self.executable = editable_viewer.executable;
                    self.arguments = editable_viewer.arguments;
                    self.selected_document_type = Some(editable_viewer.document_type);
                    widgets.name_entry.set_text(&self.name);
                    widgets.executable_entry.set_text(&self.executable);
                    self.argument_list
                        .emit(ArgumentListMsg::SetArguments(self.arguments.clone()));
                } else {
                    self.editable_viewer_id = None;
                    self.name.clear();
                    self.executable.clear();
                    self.arguments.clear();
                    self.selected_document_type = None;
                    widgets.name_entry.set_text("");
                    widgets.executable_entry.set_text("");
                    self.argument_list
                        .emit(ArgumentListMsg::SetArguments(Vec::new()));
                }
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
            DocumentViewerFormCommandMsg::DocumentViewerUpdated(Ok(id)) => {
                println!("Document viewer updated with id {}", id);
                let name = self.name.clone();
                let res = sender.output(DocumentViewerFormOutputMsg::DocumentViewerUpdated(
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
            DocumentViewerFormCommandMsg::DocumentViewerUpdated(Err(error)) => {
                eprintln!("Error in updating document_viewer: {}", error);
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
            editable_viewer_id: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}
