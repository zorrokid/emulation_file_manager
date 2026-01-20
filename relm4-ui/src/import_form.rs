use std::{path::PathBuf, sync::Arc};

use core_types::{FileType, item_type::ItemType};
use database::repository_manager::RepositoryManager;
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
use service::{
    error::Error, mass_import::service::MassImportService, view_model_service::ViewModelService,
    view_models::SystemListModel,
};
use ui_components::{DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::{
    components::item_type_dropdown::{ItemTypeDropDownOutputMsg, ItemTypeDropdown},
    system_selector::{
        SystemSelectInit, SystemSelectModel, SystemSelectMsg, SystemSelectOutputMsg,
    },
};

#[derive(Debug)]
pub struct ImportForm {
    directory_path: Option<PathBuf>,
    dat_file_path: Option<PathBuf>,
    selected_system: Option<SystemListModel>,
    selected_file_type: Option<FileType>,
    selected_item_type: Option<ItemType>,
    source: String,
    file_type_dropdown: Controller<FileTypeDropDown>,
    system_selector: Controller<SystemSelectModel>,
    mass_import_service: Arc<MassImportService>,
    item_type_dropdown_new: Controller<ItemTypeDropdown>,
}

#[derive(Debug)]
pub enum ImportFormMsg {
    OpenDirectorySelector,
    OpenFileSelector,
    DirectorySelected(PathBuf),
    DatFileSelected(PathBuf),
    FileTypeChanged(FileType),
    SourceChanged(String),
    SystemSelected(SystemListModel),
    Show,
    Hide,
    OpenSystemSelector,
    StartImport,
    ItemTypeChangedNew(Option<ItemType>),
}

#[derive(Debug)]
pub enum CommandMsg {
    ProcessImportResult(Result<(), Error>),
}

pub struct ImportFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
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
}

#[relm4::component(pub)]
impl Component for ImportForm {
    type Input = ImportFormMsg;
    type Output = ();
    type CommandOutput = CommandMsg;
    type Init = ImportFormInit;

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
                #[local_ref]
                item_type_dropdown_new -> gtk::Box,
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Button {
                       set_label: "Select folder for source files",
                       connect_clicked => ImportFormMsg::OpenDirectorySelector,
                       #[watch]
                       set_sensitive: model.selected_file_type.is_some(),
                    },

                    gtk::Label {
                        #[watch]
                        set_label: &model.directory_path.as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "No directory selected".to_string()),
                    },
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    gtk::Button {
                         set_label: "Select optional DAT file",
                         connect_clicked => ImportFormMsg::OpenFileSelector,
                     },
                    gtk::Label {
                        #[watch]
                        set_label: &model.dat_file_path.as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "No DAT file selected".to_string()),
                    },
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,
                    gtk::Button {

                        set_label: "Select System",
                        connect_clicked => ImportFormMsg::OpenSystemSelector,
                    },
                    gtk::Label {
                        #[watch]
                        set_label: &model.selected_system.as_ref()
                            .map(|s| s.name.clone())
                            .unwrap_or_else(|| "No system selected".to_string()),
                    },
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
                gtk::Button {
                    set_label: "Start Import",
                    connect_clicked => ImportFormMsg::StartImport,
                    #[watch]
                    set_sensitive: model.directory_path.is_some()
                        && model.selected_system.is_some()
                        && model.selected_file_type.is_some()
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

        let mass_import_service = Arc::new(MassImportService::new(Arc::clone(
            &init_model.repository_manager,
        )));

        let init_model = SystemSelectInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
            repository_manager: Arc::clone(&init_model.repository_manager),
        };

        let system_selector = SystemSelectModel::builder()
            .transient_for(&root)
            .launch(init_model)
            .forward(sender.input_sender(), |msg| match msg {
                SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                    ImportFormMsg::SystemSelected(system_list_model)
                }
            });

        let item_type_dropdown_new = ItemTypeDropdown::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                ItemTypeDropDownOutputMsg::ItemTypeChanged(opt_item_type) => {
                    ImportFormMsg::ItemTypeChangedNew(opt_item_type)
                }
            },
        );

        let model = ImportForm {
            directory_path: None,
            dat_file_path: None,
            selected_system: None,
            selected_file_type: None,
            selected_item_type: None,
            file_type_dropdown,
            source: String::new(),
            system_selector,
            mass_import_service,
            item_type_dropdown_new,
        };

        let file_types_dropdown = model.file_type_dropdown.widget();
        let item_type_dropdown_new = model.item_type_dropdown_new.widget();
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ImportFormMsg::OpenDirectorySelector => {
                let dialog = FileChooserDialog::builder()
                    .title("Select Directory")
                    .action(gtk::FileChooserAction::SelectFolder)
                    .modal(true)
                    .transient_for(root)
                    .build();

                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Select", gtk::ResponseType::Accept);

                dialog.connect_response(clone!(
                    #[strong]
                    sender,
                    move |dialog, response| {
                        tracing::info!("Directory selection dialog response: {:?}", response);
                        if response == gtk::ResponseType::Accept
                            && let Some(path) = dialog.file().and_then(|f| f.path())
                        {
                            tracing::info!("Selected directory path: {:?}", path);
                            sender.input(ImportFormMsg::DirectorySelected(path));
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }
            ImportFormMsg::OpenFileSelector => {
                let filter = gtk::FileFilter::new();
                filter.add_suffix("dat");
                filter.set_name(Some("DAT files"));
                let dialog = FileChooserDialog::builder()
                    .title("Select File")
                    .action(gtk::FileChooserAction::Open)
                    .filter(&filter)
                    .modal(true)
                    .transient_for(root)
                    .build();

                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Select", gtk::ResponseType::Accept);

                dialog.connect_response(clone!(
                    #[strong]
                    sender,
                    move |dialog, response| {
                        if response == gtk::ResponseType::Accept
                            && let Some(path) = dialog.file().and_then(|f| f.path())
                            // TODO: add more thorough file type checking and support for other
                            // data file types
                            && path.extension().and_then(|ext| ext.to_str()) == Some("dat")
                        {
                            sender.input(ImportFormMsg::DatFileSelected(path));
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }
            ImportFormMsg::DirectorySelected(path) => {
                tracing::info!("Directory selected: {:?}", path);
                self.directory_path = Some(path);
            }
            ImportFormMsg::DatFileSelected(path) => {
                tracing::info!("DAT file selected: {:?}", path);
                self.dat_file_path = Some(path);
            }
            ImportFormMsg::SourceChanged(source) => {
                tracing::info!("Source changed: {}", source);
                self.source = source;
            }
            ImportFormMsg::FileTypeChanged(file_type) => {
                tracing::info!("File type changed: {:?}", file_type);
                self.selected_file_type = Some(file_type);
            }
            ImportFormMsg::SystemSelected(system) => {
                tracing::info!("System selected: {:?}", system);
                self.selected_system = Some(system);
            }
            ImportFormMsg::Show => root.show(),
            ImportFormMsg::Hide => root.hide(),
            ImportFormMsg::OpenSystemSelector => {
                let selected_system_ids = if let Some(system) = &self.selected_system {
                    vec![system.id]
                } else {
                    vec![]
                };
                self.system_selector.emit(SystemSelectMsg::Show {
                    selected_system_ids,
                });
            }
            ImportFormMsg::StartImport => {
                if let (Some(selected_system), Some(directory_path), Some(file_type)) = (
                    &self.selected_system,
                    &self.directory_path,
                    self.selected_file_type,
                ) {
                    tracing::info!(
                        "Starting import with file_path: {:?}, source: {}, file_type: {:?}, item_type: {:?}, system: {:?}",
                        self.directory_path,
                        self.source,
                        self.selected_file_type,
                        self.selected_item_type,
                        selected_system,
                    );

                    let dat_file_path = self.dat_file_path.clone();
                    let directory_path = directory_path.clone();
                    let system_id = selected_system.id;
                    let mass_import_service = Arc::clone(&self.mass_import_service);
                    sender.oneshot_command(async move {
                        let result = mass_import_service
                            .import(system_id, directory_path, dat_file_path, file_type)
                            .await;
                        CommandMsg::ProcessImportResult(result)
                    });
                }
            }
            ImportFormMsg::ItemTypeChangedNew(opt_item_type) => {
                tracing::info!("Item type changed (new component): {:?}", opt_item_type);
                self.selected_item_type = opt_item_type;
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::ProcessImportResult(result) => match result {
                Ok(_) => {
                    tracing::info!("Import completed successfully.");
                    root.hide();
                }
                Err(e) => {
                    tracing::error!("Import failed: {:?}", e);
                    // Here you could show an error dialog to the user
                }
            },
        }
    }
}
