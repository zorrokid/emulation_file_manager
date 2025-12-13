use std::sync::Arc;

use core_types::FileType;
use database::{database_error::DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
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
    view_models::{FileSetListModel, FileSetViewModel},
};
use ui_components::{DropDownMsg, DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::utils::dialog_utils::show_error_dialog;

#[derive(Debug)]
pub struct FileSetEditor {
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    file_set_id: Option<i64>,
    selected_file_type: Option<FileType>,
    file_set_name: String,
    file_set_file_name: String,
    source: String,
    dropdown: Controller<FileTypeDropDown>,
}

#[derive(Debug)]
pub enum FileSetEditorMsg {
    Show { file_set_id: i64 },
    SaveChanges,
    FileSetFileNameChanged(String),
    FileSetNameChanged(String),
    SourceChanged(String),
    FileTypeChanged(FileType),
    Hide,
    UpdateFormFields,
}

#[derive(Debug)]
pub enum FileSetEditorOutputMsg {
    FileSetUpdated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FileSetFetched(Result<FileSetViewModel, service::error::Error>),
    FileSetUpdated(Result<i64, DatabaseError>, FileSetListModel),
}

pub struct FileSetEditorInit {
    pub view_model_service: Arc<service::view_model_service::ViewModelService>,
    pub repository_manager: Arc<database::repository_manager::RepositoryManager>,
}

#[relm4::component(pub)]
impl Component for FileSetEditor {
    type Input = FileSetEditorMsg;
    type Output = FileSetEditorOutputMsg;
    type Init = FileSetEditorInit;
    type CommandOutput = CommandMsg;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            set_margin_all: 10,
            set_title: Some("Edit File Set names"),

            connect_close_request[sender] => move |_| {
                sender.input(FileSetEditorMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "File Type:",
                    },

                    #[local_ref]
                    file_types_dropdown -> gtk::Box,
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "File Set File Name:",
                    },

                    #[name = "file_name_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("File Set File Name"),
                        set_text: &model.file_set_file_name,
                        connect_activate[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(
                                FileSetEditorMsg::FileSetFileNameChanged(buffer.text().into()),
                            );
                        }
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "File Set Display Name:",
                    },
                    #[name = "name_entry"]
                    gtk::Entry {
                        set_placeholder_text: Some("File Set Display Name"),
                        set_text: &model.file_set_name,
                        connect_activate[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(
                                FileSetEditorMsg::FileSetNameChanged(buffer.text().into()),
                            );
                        }
                    },
                },

                #[name = "source_entry"]
                gtk::Entry {
                    set_placeholder_text: Some("Source (e.g. website URL)"),
                    set_text: &model.source,
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            FileSetEditorMsg::SourceChanged(buffer.text().into()),
                        );
                    }
                },

               gtk::Button {
                    set_label: "Update File Set",
                    connect_clicked => FileSetEditorMsg::SaveChanges,
                    #[watch]
                    set_sensitive: model.file_set_id.is_some() && model.selected_file_type.is_some(),
               },
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
            FileSetEditorMsg::Show { file_set_id } => {
                root.show();
                self.file_set_id = Some(file_set_id);
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let result = view_model_service
                        .get_file_set_view_model(file_set_id)
                        .await;
                    CommandMsg::FileSetFetched(result)
                });
            }
            FileSetEditorMsg::Hide => {
                root.hide();
            }
            FileSetEditorMsg::FileSetFileNameChanged(file_name) => {
                self.file_set_file_name = file_name;
            }
            FileSetEditorMsg::FileSetNameChanged(name) => {
                self.file_set_name = name;
            }
            FileSetEditorMsg::SourceChanged(source) => {
                self.source = source;
            }
            FileSetEditorMsg::FileTypeChanged(file_type) => {
                self.selected_file_type = Some(file_type);
            }
            FileSetEditorMsg::SaveChanges => {
                tracing::info!("Saving changes to file set");
                if let (Some(file_set_id), Some(file_type)) =
                    (self.file_set_id, self.selected_file_type)
                {
                    tracing::info!(id = file_set_id, "Updating file set",);
                    let repository_manager = Arc::clone(&self.repository_manager);
                    let file_set_name = self.file_set_name.clone();
                    let file_set_file_name = self.file_set_file_name.clone();
                    let source = self.source.clone();
                    let file_set_list_model = FileSetListModel {
                        id: file_set_id,
                        file_set_name: file_set_name.clone(),
                        file_name: file_set_file_name.clone(),
                        file_type,
                        can_delete: true,
                    };
                    sender.oneshot_command(async move {
                        let res = repository_manager
                            .get_file_set_repository()
                            .update_file_set(
                                file_set_id,
                                &file_set_file_name,
                                &file_set_name,
                                &source,
                                &file_type,
                            )
                            .await;
                        CommandMsg::FileSetUpdated(res, file_set_list_model)
                    });
                }
            }
            FileSetEditorMsg::UpdateFormFields => {
                widgets.file_name_entry.set_text(&self.file_set_file_name);
                widgets.name_entry.set_text(&self.file_set_name);
                widgets.source_entry.set_text(&self.source);
            }
        }
        // This is essential with update_with_view:
        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::FileSetFetched(Ok(file_set)) => {
                self.file_set_name = file_set.file_set_name.clone();
                self.file_set_file_name = file_set.file_name.clone();
                self.source = file_set.source.clone();
                self.selected_file_type = Some(file_set.file_type);
                self.dropdown
                    .emit(DropDownMsg::SetSelected(file_set.file_type));
                sender.input(FileSetEditorMsg::UpdateFormFields);
            }
            CommandMsg::FileSetFetched(Err(e)) => {
                tracing::error!( error = ?e, "Error fetching file set");
                show_error_dialog(format!("Error fetching file set: {:?}", e), root);
            }
            CommandMsg::FileSetUpdated(Ok(_), file_set_list_model) => {
                tracing::info!("File set updated successfully");
                sender
                    .output(FileSetEditorOutputMsg::FileSetUpdated(file_set_list_model))
                    .unwrap_or_else(|e| {
                        tracing::error!(
                            error = ?e,
                            "Error sending FileSetUpdated output message");
                    });

                root.close();
            }
            CommandMsg::FileSetUpdated(Err(e), _) => {
                tracing::error!(error = ?e, "Error updating file set");
                show_error_dialog(format!("Error updating file set: {:?}", e), root);
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let dropdown = Self::create_dropdown(None, &sender);

        let model = FileSetEditor {
            file_set_id: None,
            selected_file_type: None,
            file_set_name: String::new(),
            file_set_file_name: String::new(),
            source: String::new(),
            repository_manager: init.repository_manager,
            view_model_service: init.view_model_service,
            dropdown,
        };
        let file_types_dropdown = model.dropdown.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

impl FileSetEditor {
    fn create_dropdown(
        initial_selection: Option<FileType>,
        sender: &ComponentSender<Self>,
    ) -> Controller<FileTypeDropDown> {
        FileTypeDropDown::builder()
            .launch(initial_selection)
            .forward(sender.input_sender(), |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(
                    file_type,
                )) => FileSetEditorMsg::FileTypeChanged(file_type),
                _ => unreachable!(),
            })
    }
}
