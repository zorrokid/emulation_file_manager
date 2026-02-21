use std::sync::Arc;

use core_types::{ArgumentType, DocumentType};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, CheckButtonExt, EditableExt, EntryBufferExtManual, EntryExt,
            GtkWindowExt, OrientableExt, WidgetExt,
        },
    },
};
use service::{
    error::Error,
    view_models::{DocumentViewerListModel, DocumentViewerViewModel},
};
use ui_components::{
    DropDownOutputMsg,
    drop_down::{DocumentTypeDropDown, DocumentTypeSelectedMsg},
};

use crate::{
    argument_list::{ArgumentList, ArgumentListMsg, ArgumentListOutputMsg},
    utils::dialog_utils::show_error_dialog,
};

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
    DocumentTypeChanged(DocumentType),
    CleanupTempFilesToggled(bool),
}

#[derive(Debug)]
pub enum DocumentViewerFormOutputMsg {
    DocumentViewerAdded(DocumentViewerListModel),
    DocumentViewerUpdated(DocumentViewerListModel),
}

#[derive(Debug)]
pub enum DocumentViewerFormCommandMsg {
    DocumentViewerSubmitted(Result<i64, Error>),
    DocumentViewerUpdated(Result<i64, Error>),
}

pub struct DocumentViewerFormInit {
    pub app_services: Arc<service::app_services::AppServices>,
}

#[derive(Debug)]
pub struct DocumentViewerFormModel {
    pub app_services: Arc<service::app_services::AppServices>,
    pub name: String,
    pub executable: String,
    pub selected_document_type: Option<DocumentType>,
    argument_list: Controller<ArgumentList>,
    arguments: Vec<ArgumentType>,
    editable_viewer_id: Option<i64>,
    dropdown: Controller<DocumentTypeDropDown>,
    pub cleanup_temp_files: bool,
}

#[relm4::component(pub)]
impl Component for DocumentViewerFormModel {
    type Input = DocumentViewerFormMsg;
    type Output = DocumentViewerFormOutputMsg;
    type CommandOutput = DocumentViewerFormCommandMsg;
    type Init = DocumentViewerFormInit;

    view! {
        gtk::Window {
            set_title: Some("Document Viewer Form"),

            connect_close_request[sender] => move |_| {
                sender.input(DocumentViewerFormMsg::Hide);
                glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 6,

                gtk::Box {
                   set_orientation: gtk::Orientation::Horizontal,
                   set_spacing: 6,

                   gtk::Label {
                       set_label: "Name",
                   },

                   #[name = "name_entry"]
                   gtk::Entry {
                       set_text: &model.name,
                       set_placeholder_text: Some("DocumentViewer name"),
                       set_hexpand: true,
                       connect_changed[sender] => move |entry| {
                           let buffer = entry.buffer();
                           sender.input(
                               DocumentViewerFormMsg::NameChanged(buffer.text().into()),
                           );
                       }
                   },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    gtk::Label {
                        set_label: "Executable",
                    },

                    #[name = "executable_entry"]
                    gtk::Entry {
                        set_text: &model.executable,
                        set_placeholder_text: Some("DocumentViewer executable"),
                        set_hexpand: true,
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(
                                DocumentViewerFormMsg::ExecutableChanged(buffer.text().into()),
                            );
                        },
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    gtk::Label {
                        set_label: "Document Type",
                    },
                    #[local_ref]
                    document_types_dropdown -> gtk::Box{
                        set_hexpand: true,
                    },
                },

                gtk::Box {
                    append = model.argument_list.widget(),
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,

                    #[name = "cleanup_checkbox"]
                    gtk::CheckButton {
                        set_label: Some("Cleanup temporary files after launch"),
                        #[watch]
                        #[block_signal(cleanup_temp_files_toggled)]
                        set_active: model.cleanup_temp_files,
                        set_tooltip_text: Some("Enable if viewer blocks until closed. Disable for spawning viewers like xdg-open."),
                        connect_toggled[sender] => move |checkbox| {
                            sender.input(DocumentViewerFormMsg::CleanupTempFilesToggled(checkbox.is_active()));
                        } @cleanup_temp_files_toggled,
                    },
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
                self.executable = executable;
            }
            DocumentViewerFormMsg::Submit => {
                if let Some(document_type) = self.selected_document_type {
                    let executable = self.executable.clone();
                    let name = self.name.clone();
                    let arguments = self.arguments.clone();
                    let cleanup_temp_files = self.cleanup_temp_files;

                    let app_services = Arc::clone(&self.app_services);

                    if let Some(editable_id) = self.editable_viewer_id {
                        sender.oneshot_command(async move {
                            let res = app_services
                                .document_viewer
                                .update_document_viewer(
                                    editable_id,
                                    &name,
                                    &executable,
                                    &arguments,
                                    &document_type,
                                    cleanup_temp_files,
                                )
                                .await;
                            DocumentViewerFormCommandMsg::DocumentViewerUpdated(res)
                        });
                    } else {
                        sender.oneshot_command(async move {
                            let res = app_services
                                .document_viewer
                                .add_document_viewer(
                                    &name,
                                    &executable,
                                    &arguments,
                                    &document_type,
                                    cleanup_temp_files,
                                )
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
                    self.editable_viewer_id = Some(editable_viewer.id);
                    self.name = editable_viewer.name;
                    self.executable = editable_viewer.executable;
                    self.arguments = editable_viewer.arguments;
                    self.selected_document_type = Some(editable_viewer.document_type);
                    self.cleanup_temp_files = editable_viewer.cleanup_temp_files;
                    widgets.name_entry.set_text(&self.name);
                    widgets.executable_entry.set_text(&self.executable);
                    widgets.cleanup_checkbox.set_active(self.cleanup_temp_files);
                    self.argument_list
                        .emit(ArgumentListMsg::SetArguments(self.arguments.clone()));
                } else {
                    self.editable_viewer_id = None;
                    self.name.clear();
                    self.executable.clear();
                    self.arguments.clear();
                    self.selected_document_type = None;
                    self.cleanup_temp_files = false;
                    widgets.name_entry.set_text("");
                    widgets.executable_entry.set_text("");
                    widgets.cleanup_checkbox.set_active(false);
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
            DocumentViewerFormMsg::DocumentTypeChanged(document_type) => {
                self.selected_document_type = Some(document_type);
            }
            DocumentViewerFormMsg::CleanupTempFilesToggled(value) => {
                self.cleanup_temp_files = value;
            }
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
                let name = self.name.clone();
                sender
                    .output(DocumentViewerFormOutputMsg::DocumentViewerAdded(
                        DocumentViewerListModel { id, name },
                    ))
                    .unwrap_or_else(|error| {
                        tracing::error!(
                        error = ?error,
                        "Sending message failed");
                    });

                root.close();
            }
            DocumentViewerFormCommandMsg::DocumentViewerSubmitted(Err(error)) => {
                tracing::error!(
                    error = ?error,
                    "Error in submitting document_viewer");
                show_error_dialog(
                    format!("Error in submitting document_viewer: {}", error),
                    root,
                );
            }
            DocumentViewerFormCommandMsg::DocumentViewerUpdated(Ok(id)) => {
                let name = self.name.clone();
                sender
                    .output(DocumentViewerFormOutputMsg::DocumentViewerUpdated(
                        DocumentViewerListModel { id, name },
                    ))
                    .unwrap_or_else(|error| {
                        tracing::error!(
                    error = ?error,
                    "Sending message failed");
                    });

                root.close();
            }
            DocumentViewerFormCommandMsg::DocumentViewerUpdated(Err(error)) => {
                tracing::error!(
                    error = ?error,
                    "Error in updating document_viewer");
                show_error_dialog(
                    format!("Error in updating document_viewer: {}", error),
                    root,
                );
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
                });

        let dropdown = Self::create_dropdown(None, &sender);

        let model = Self {
            app_services: init.app_services,
            executable: String::new(),
            arguments: Vec::new(),
            name: String::new(),
            selected_document_type: None,
            argument_list,
            editable_viewer_id: None,
            dropdown,
            cleanup_temp_files: false,
        };

        let document_types_dropdown = model.dropdown.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

impl DocumentViewerFormModel {
    fn create_dropdown(
        initial_selection: Option<DocumentType>,
        sender: &ComponentSender<Self>,
    ) -> Controller<DocumentTypeDropDown> {
        DocumentTypeDropDown::builder()
            .launch(initial_selection)
            .forward(sender.input_sender(), |msg| match msg {
                DropDownOutputMsg::ItemSelected(DocumentTypeSelectedMsg::DocumentTypeSelected(
                    document_type,
                )) => DocumentViewerFormMsg::DocumentTypeChanged(document_type),
                _ => unreachable!(),
            })
    }
}
