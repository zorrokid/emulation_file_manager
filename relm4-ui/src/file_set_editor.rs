use std::sync::Arc;

use core_types::FileType;
use database::repository_manager::RepositoryManager;
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
    Hide,
    SaveChanges,
    FileSetFileNameChanged(String),
    FileSetNameChanged(String),
    SourceChanged(String),
    FileTypeChanged(FileType),
}

#[derive(Debug)]
pub enum FileSetEditorOutputMsg {
    FileSetUpdated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FileSetFetched(Result<FileSetViewModel, service::error::Error>),
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
            set_default_width: 400,
            set_default_height: 300,
            set_title: Some("Edit File Set"),
            connect_close_request[sender] => move |_| {
                sender.input(FileSetEditorMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,

                #[local_ref]
                file_types_dropdown -> gtk::Box,


                gtk::Entry {
                    set_placeholder_text: Some("File Set File Name"),
                    #[watch]
                    set_text: &model.file_set_file_name,
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            FileSetEditorMsg::FileSetFileNameChanged(buffer.text().into()),
                        );
                    }
                },


                gtk::Entry {
                    set_placeholder_text: Some("File Set Description"),
                    #[watch]
                    set_text: &model.file_set_name,
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            FileSetEditorMsg::FileSetNameChanged(buffer.text().into()),
                        );
                    }
                },

                gtk::Entry {
                    set_placeholder_text: Some("Source (e.g. website URL)"),
                    #[watch]
                    set_text: &model.source,
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            FileSetEditorMsg::SourceChanged(buffer.text().into()),
                        );
                    }
                },

                gtk::Button {
                    set_label: "Save Changes",
                    connect_clicked => FileSetEditorMsg::SaveChanges,
                },
            }

            // Add your widgets here
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let drop_down = FileTypeDropDown::builder().launch(None).forward(
            sender.input_sender(),
            |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(
                    file_type,
                )) => FileSetEditorMsg::FileTypeChanged(file_type),
                _ => unreachable!(),
            },
        );

        let model = FileSetEditor {
            file_set_id: None,
            selected_file_type: None,
            file_set_name: String::new(),
            file_set_file_name: String::new(),
            source: String::new(),
            repository_manager: init.repository_manager,
            view_model_service: init.view_model_service,
            dropdown: drop_down,
        };

        let file_types_dropdown = model.dropdown.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
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
            _ => (),
        }
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
                self.selected_file_type = Some(file_set.file_type.into());
                self.dropdown
                    .emit(DropDownMsg::SetSelected(file_set.file_type.into()));
            }
            CommandMsg::FileSetFetched(Err(e)) => {
                eprintln!("Error fetching file set: {:?}", e);
            }
        }
    }
}
