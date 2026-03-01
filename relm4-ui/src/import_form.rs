use std::{path::PathBuf, sync::Arc};

use async_std::{channel::unbounded, task};
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
    typed_view::list::{RelmListItem, TypedListView},
};
use service::{
    error::Error,
    mass_import::models::{
        FileSetImportStatus, MassImportInput, MassImportSyncEvent, MassImportWithDatFileResult,
    },
    view_models::SystemListModel,
};
use ui_components::{DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::{
    components::item_type_dropdown::{ItemTypeDropDownOutputMsg, ItemTypeDropdown},
    system_selector::{
        SystemSelectInit, SystemSelectModel, SystemSelectMsg, SystemSelectOutputMsg,
    },
    utils::dialog_utils::show_error_dialog,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportListItem {
    pub name: String,
}

pub struct ListItemWidgets {
    label: gtk::Label,
}

impl RelmListItem for ImportListItem {
    type Root = gtk::Box;
    type Widgets = ListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, ListItemWidgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                #[name = "label"]
                gtk::Label,
            }
        }

        let widgets = ListItemWidgets { label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let ListItemWidgets { label } = widgets;
        label.set_label(self.name.as_str());
    }
}

#[derive(Debug)]
pub struct ImportForm {
    app_services: Arc<service::app_services::AppServices>,
    directory_path: Option<PathBuf>,
    dat_file_path: Option<PathBuf>,
    selected_system: Option<SystemListModel>,
    selected_file_type: Option<FileType>,
    selected_item_type: Option<ItemType>,
    source: String,
    file_type_dropdown: Controller<FileTypeDropDown>,
    system_selector: Controller<SystemSelectModel>,
    item_type_dropdown: Controller<ItemTypeDropdown>,
    imported_sets_list_view_wrapper: TypedListView<ImportListItem, gtk::NoSelection>,
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
    ItemTypeChanged(Option<ItemType>),
    ProcessSyncEvent(MassImportSyncEvent),
    ImportSummaryDialogClosed,
}

#[derive(Debug)]
pub enum CommandMsg {
    ProcessImportResult(Result<MassImportWithDatFileResult, Error>),
}

pub struct ImportFormInit {
    pub app_services: Arc<service::app_services::AppServices>,
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
                item_type_dropdown -> gtk::Box,
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
                    set_text: &model.source,
                    connect_changed[sender] => move |entry| {
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
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    imported_files_list -> gtk::ListView {},
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

        let imported_sets_list_view_wrapper =
            TypedListView::<ImportListItem, gtk::NoSelection>::new();
        let app_services = Arc::clone(&init_model.app_services);

        let init_model = SystemSelectInit {
            app_services: Arc::clone(&init_model.app_services),
        };

        let system_selector = SystemSelectModel::builder()
            .transient_for(&root)
            .launch(init_model)
            .forward(sender.input_sender(), |msg| match msg {
                SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                    ImportFormMsg::SystemSelected(system_list_model)
                }
            });

        let item_type_dropdown = ItemTypeDropdown::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                ItemTypeDropDownOutputMsg::ItemTypeChanged(opt_item_type) => {
                    ImportFormMsg::ItemTypeChanged(opt_item_type)
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
            item_type_dropdown,
            imported_sets_list_view_wrapper,
            app_services,
        };

        let file_types_dropdown = model.file_type_dropdown.widget();
        let item_type_dropdown = model.item_type_dropdown.widget();
        let imported_files_list = &model.imported_sets_list_view_wrapper.view;
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
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
            ImportFormMsg::Hide => {
                self.imported_sets_list_view_wrapper.clear();
                root.hide()
            }
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

                    let mass_import_service = self.app_services.import().clone();

                    let input = MassImportInput {
                        source_path: directory_path.clone(),
                        dat_file_path: self.dat_file_path.clone(),
                        file_type,
                        item_type: self.selected_item_type,
                        system_id: selected_system.id,
                    };
                    let (progress_tx, progress_rx) = unbounded::<MassImportSyncEvent>();
                    let ui_sender = sender.clone();

                    // Spawn task to forward progress messages to UI
                    task::spawn(async move {
                        while let Ok(event) = progress_rx.recv().await {
                            ui_sender.input(ImportFormMsg::ProcessSyncEvent(event));
                        }
                    });

                    sender.oneshot_command(async move {
                        let result = mass_import_service
                            .import_with_dat(input, Some(progress_tx))
                            .await;
                        CommandMsg::ProcessImportResult(result)
                    });
                }
            }
            ImportFormMsg::ItemTypeChanged(opt_item_type) => {
                tracing::info!(
                    opt_item_type = ?opt_item_type,
                    "Item type changed (new component)");
                self.selected_item_type = opt_item_type;
            }
            ImportFormMsg::ProcessSyncEvent(event) => {
                tracing::info!(event = ?event, "Received sync event");
                self.imported_sets_list_view_wrapper.append(ImportListItem {
                    name: event.file_set_name,
                });
            }
            ImportFormMsg::ImportSummaryDialogClosed => {
                tracing::info!("Import summary dialog closed, hiding form");
                self.imported_sets_list_view_wrapper.clear();
                root.hide();
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
            CommandMsg::ProcessImportResult(result) => match result {
                Ok(mass_import_result) => {
                    self.show_import_summary_dialog(&mass_import_result, root, &sender);
                }
                Err(e) => {
                    tracing::error!("Import failed: {:?}", e);
                    show_error_dialog(format!("The import process failed: {:?}", e), root);
                }
            },
        }
    }
}

impl ImportForm {
    fn show_import_summary_dialog(
        &self,
        result: &MassImportWithDatFileResult,
        root: &gtk::Window,
        sender: &ComponentSender<Self>,
    ) {
        // TODO: improve the summary details
        let successful_imports = result
            .result
            .import_results
            .iter()
            .filter(|r| matches!(r.status, FileSetImportStatus::Success))
            .count();
        let failed_imports = result
            .result
            .import_results
            .iter()
            .filter(|r| matches!(r.status, FileSetImportStatus::Failed(_)))
            .collect::<Vec<_>>();
        let successful_with_warnings = result
            .result
            .import_results
            .iter()
            .filter(|r| matches!(r.status, FileSetImportStatus::SucessWithWarnings(_)))
            .collect::<Vec<_>>();
        let error_messages = failed_imports
            .iter()
            .map(|r| {
                if let FileSetImportStatus::Failed(err_msg) = &r.status {
                    err_msg.clone()
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let warning_messages = successful_with_warnings
            .iter()
            .flat_map(|r| {
                if let FileSetImportStatus::SucessWithWarnings(warnings) = &r.status {
                    warnings.clone()
                } else {
                    vec![]
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let message = format!(
            "Import Summary:\n\
            Successful imports: {}\n\
            Imports with warnings: {}\n\
            Failed imports: {}\n\n\
            {}\
            {}",
            successful_imports,
            successful_with_warnings.len(),
            failed_imports.len(),
            if !warning_messages.is_empty() {
                format!("Warnings:\n{}\n\n", warning_messages)
            } else {
                String::new()
            },
            if !error_messages.is_empty() {
                format!("Errors:\n{}", error_messages)
            } else {
                String::new()
            },
        );

        let dialog = gtk::MessageDialog::new(
            Some(root),
            gtk::DialogFlags::MODAL,
            gtk::MessageType::Info,
            gtk::ButtonsType::Ok,
            &message,
        );
        dialog.connect_response(clone!(
            #[strong]
            sender,
            move |dialog, _| {
                dialog.close();
                sender.input(ImportFormMsg::ImportSummaryDialogClosed);
            }
        ));
        dialog.show();
    }
}
