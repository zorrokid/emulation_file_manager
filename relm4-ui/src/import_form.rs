use std::path::PathBuf;

use core_types::{FileType, item_type::ItemType};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self, FileChooserDialog,
        gio::prelude::FileExt,
        glib::{self, clone},
        prelude::{
            BoxExt, ButtonExt, DialogExt, EditableExt, EntryBufferExtManual, EntryExt,
            FileChooserExt, GtkWindowExt, OrientableExt, WidgetExt,
        },
    },
};
use ui_components::{
    DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg,
    drop_down::{ItemTypeDropDown, ItemTypeSelectedMsg},
};

#[derive(Debug)]
pub struct ImportForm {
    file_path: PathBuf,
    selected_system_id: Option<i32>,
    selected_file_type: Option<FileType>,
    selected_item_type: Option<ItemType>,
    source: String,
    file_type_dropdown: Controller<FileTypeDropDown>,
    item_type_dropdown: Controller<ItemTypeDropDown>,
}

#[derive(Debug)]
pub enum ImportFormMsg {
    OpenFileSelector,
    DirectorySelected(PathBuf),
    FileTypeChanged(FileType),
    ItemTypeChanged(ItemType),
    SourceChanged(String),
    Show,
    Hide,
}

impl ImportForm {
    fn create_file_type_dropdown(
        initial_selection: Option<FileType>,
        sender: &ComponentSender<Self>,
    ) -> Controller<FileTypeDropDown> {
        FileTypeDropDown::builder()
            .launch(initial_selection)
            .forward(sender.input_sender(), |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(
                    file_type,
                )) => ImportFormMsg::FileTypeChanged(file_type),
                _ => unreachable!(),
            })
    }

    fn create_item_type_dropdown(
        initial_selection: Option<ItemType>,
        sender: &ComponentSender<Self>,
    ) -> Controller<ItemTypeDropDown> {
        ItemTypeDropDown::builder()
            .launch(initial_selection)
            .forward(sender.input_sender(), |msg| match msg {
                DropDownOutputMsg::ItemSelected(ItemTypeSelectedMsg::ItemTypeSelected(
                    file_type,
                )) => ImportFormMsg::ItemTypeChanged(file_type),
                _ => unreachable!(),
            })
    }
}

#[relm4::component(pub)]
impl Component for ImportForm {
    type Input = ImportFormMsg;
    type Output = ();
    type CommandOutput = ();
    type Init = ();

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            set_margin_all: 10,
            set_title: Some("Import"),
            connect_close_request[sender] => move |_| {
                sender.input(ImportFormMsg::Hide);
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
                        set_label: "Item Type:",
                    },

                    #[local_ref]
                    item_types_dropdown -> gtk::Box,
                },
                 gtk::Button {
                    set_label: "Open File Picker",
                    connect_clicked => ImportFormMsg::OpenFileSelector,
                    #[watch]
                    set_sensitive: model.selected_file_type.is_some(),
                },
                gtk::Entry {
                    set_placeholder_text: Some("Source (e.g. website URL)"),
                    #[watch]
                    set_text: &model.source,
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            ImportFormMsg::SourceChanged(buffer.text().into()),
                        );
                    }
                },
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let file_type_dropdown = Self::create_file_type_dropdown(None, &sender);
        let item_type_dropdown = Self::create_item_type_dropdown(None, &sender);
        let model = ImportForm {
            file_path: PathBuf::new(),
            selected_system_id: None,
            selected_file_type: None,
            selected_item_type: None,
            file_type_dropdown,
            item_type_dropdown,
            source: String::new(),
        };

        let file_types_dropdown = model.file_type_dropdown.widget();
        let item_types_dropdown = model.item_type_dropdown.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ImportFormMsg::OpenFileSelector => {
                let dialog = FileChooserDialog::builder()
                    .title("Select Files")
                    .action(gtk::FileChooserAction::Open)
                    .modal(true)
                    .transient_for(root)
                    .build();

                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Open", gtk::ResponseType::Accept);

                dialog.connect_response(clone!(
                    #[strong]
                    sender,
                    move |dialog, response| {
                        if response == gtk::ResponseType::Accept
                            && let Some(path) = dialog.file().and_then(|f| f.path())
                        {
                            sender.input(ImportFormMsg::DirectorySelected(path));
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }
            ImportFormMsg::DirectorySelected(path) => {
                self.file_path = path;
            }
            ImportFormMsg::SourceChanged(source) => {
                self.source = source;
            }
            ImportFormMsg::FileTypeChanged(file_type) => {
                self.selected_file_type = Some(file_type);
            }
            ImportFormMsg::ItemTypeChanged(item_type) => {
                self.selected_item_type = Some(item_type);
            }
            ImportFormMsg::Show => root.show(),
            ImportFormMsg::Hide => root.hide(),
        }
    }
}
