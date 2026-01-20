use std::{path::PathBuf, sync::Arc};

use async_std::{channel::unbounded, task};
use core_types::{
    FileType, ReadFile, Sha1Checksum, events::HttpDownloadEvent, item_type::ItemType,
};
use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, FactorySender,
    RelmWidgetExt,
    gtk::{
        self, FileChooserDialog,
        gio::prelude::FileExt,
        glib::{self, clone},
        prelude::{
            BoxExt, ButtonExt, CheckButtonExt, DialogExt, EditableExt, EntryBufferExtManual,
            EntryExt, FileChooserExt, GtkWindowExt, OrientableExt, WidgetExt,
        },
    },
    prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque},
};
use service::{
    download_service::DownloadService,
    error::Error,
    file_import::{
        model::{
            FileImportPrepareResult, FileImportResult, FileImportSource, FileSetImportModel,
            UpdateFileSetModel,
        },
        service::FileImportService,
    },
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, FileSetViewModel, Settings},
};
use ui_components::{DropDownMsg, DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::{
    components::item_type_dropdown::{
        ItemTypeDropDownMsg, ItemTypeDropDownOutputMsg, ItemTypeDropdown,
    },
    utils::{dialog_utils::show_error_dialog, string_utils::format_bytes},
};

#[derive(Debug, Clone)]
struct File {
    name: String,
    sha1_checksun: Sha1Checksum,
    selected: bool,
}

#[derive(Debug)]
enum FileInput {
    Toggle(bool),
}

#[derive(Debug)]
enum FileOutput {
    SetFileSelected {
        sha1_checksum: Sha1Checksum,
        selected: bool,
    },
}

#[relm4::factory]
impl FactoryComponent for File {
    type Init = ReadFile;
    type Input = FileInput;
    type Output = FileOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,

            gtk::CheckButton {
                set_active: self.selected,
                set_margin_all: 12,
                connect_toggled[sender, sha1_checksum = self.sha1_checksun.clone()] => move |checkbox| {
                    sender.input(FileInput::Toggle(checkbox.is_active()));
                    sender.output(FileOutput::SetFileSelected {
                        sha1_checksum,
                        selected: checkbox.is_active(),
                    }).unwrap_or_else(|e| {
                        tracing::error!(error = ?e, "Error sending output");
                    });
                }
            },

            #[name(label)]
            gtk::Label {
                set_label: &self.name,
                set_hexpand: true,
                set_halign: gtk::Align::Start,
                set_margin_all: 12,
            },
        }
    }

    fn pre_view() {}

    fn init_model(
        read_file: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self {
            name: read_file.file_name,
            sha1_checksun: read_file.sha1_checksum,
            selected: true, // initially all files are selected for import
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            FileInput::Toggle(selected) => {
                self.selected = selected;
            }
        }
    }
}

#[derive(Debug)]
pub enum FileSetFormMsg {
    OpenFileSelector,
    FileSelected(PathBuf),
    OpenDownloadDialog,
    DownloadFromUrl(String),
    CreateOrUpdateFileSet,
    SetFileSelected {
        sha1_checksum: Sha1Checksum,
        selected: bool,
    },
    FileSetNameChanged(String),
    FileSetFileNameChanged(String),
    SourceChanged(String),
    FileTypeChanged(FileType),
    Show {
        selected_system_ids: Vec<i64>,
        selected_file_type: FileType,
    },
    ShowEdit {
        file_set_id: i64,
    },
    Hide,
    ProcessDownloadEvent(HttpDownloadEvent),
    CancelDownload,
    ItemTypeChanged(Option<ItemType>),
}

#[derive(Debug)]
pub enum FileSetFormOutputMsg {
    FileSetCreated(FileSetListModel),
    FileSetUpdated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FileImportPrepared(Result<FileImportPrepareResult, Error>),
    ProcessCreateOrUpdateFileSetResult(Result<FileImportResult, Error>),
    ProcessFileSetResponse(Result<FileSetViewModel, Error>),
}

pub struct FileSetFormInit {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub view_model_service: Arc<ViewModelService>,
}

#[derive(Debug)]
pub struct FileSetFormModel {
    files: FactoryVecDeque<File>,
    selected_system_ids: Vec<i64>,
    file_set_name: String,
    file_set_file_name: String,
    source: String,
    file_set_id: Option<i64>,

    dropdown: Controller<FileTypeDropDown>,
    processing: bool,
    file_import_service: Arc<FileImportService>,
    download_service: Arc<DownloadService>,
    view_model_service: Arc<ViewModelService>,
    settings: Arc<Settings>,
    selected_file_type: Option<FileType>,
    selected_files_in_picked_files: Vec<Sha1Checksum>,
    /// This contains newly picked files for import
    picked_files: Vec<FileImportSource>,
    // Download progress tracking
    download_in_progress: bool,
    download_total_size: Option<u64>,
    download_bytes: u64,
    download_cancel_tx: Option<async_std::channel::Sender<()>>,
    item_type_dropdown: Controller<ItemTypeDropdown>,
    selected_item_type: Option<ItemType>,
}

impl FileSetFormModel {
    fn create_dropdown(
        initial_selection: Option<FileType>,
        sender: &ComponentSender<Self>,
    ) -> Controller<FileTypeDropDown> {
        FileTypeDropDown::builder()
            .launch(initial_selection)
            .forward(sender.input_sender(), |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(
                    file_type,
                )) => FileSetFormMsg::FileTypeChanged(file_type),
                _ => unreachable!(),
            })
    }
}

#[relm4::component(pub)]
impl Component for FileSetFormModel {
    type Input = FileSetFormMsg;
    type Output = FileSetFormOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSetFormInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            set_margin_all: 10,
            #[watch]
            set_title: if model.file_set_id.is_some() {
                Some("Edit File Set")
            } else {
                Some("Create File Set")
            },

            connect_close_request[sender] => move |_| {
                sender.input(FileSetFormMsg::Hide);
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

                // TODO: add item type selection
                // and ensure that item type is selected for certain file types

                gtk::Button {
                    set_label: "Open File Picker",
                    connect_clicked => FileSetFormMsg::OpenFileSelector,
                    #[watch]
                    set_sensitive: model.selected_file_type.is_some(),
                },

                gtk::Button {
                    set_label: "Download from URL",
                    connect_clicked => FileSetFormMsg::OpenDownloadDialog,
                    #[watch]
                    set_sensitive: model.selected_file_type.is_some(),
                },

                // Download progress bar
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 5,
                    #[watch]
                    set_visible: model.download_in_progress,

                    gtk::Label {
                        #[watch]
                        set_label: &if let Some(total) = model.download_total_size && total > 0 {
                            format!(
                                "Downloading: {} / {} ({:.1}%)",
                                format_bytes(model.download_bytes),
                                format_bytes(total),
                                (model.download_bytes as f64 / total as f64) * 100.0
                            )
                        } else {
                            format!("Downloading: {}", format_bytes(model.download_bytes))
                        },
                        set_halign: gtk::Align::Start,
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_spacing: 5,

                        gtk::ProgressBar {
                            set_hexpand: true,
                            #[watch]
                            set_fraction: if let Some(total) = model.download_total_size {
                                (model.download_bytes as f64 / total as f64).min(1.0)
                            } else {
                                0.0
                            },
                            #[watch]
                            set_pulse_step: if model.download_total_size.is_none() { 0.1 } else { 0.0 },
                        },

                        gtk::Button {
                            set_label: "Cancel",
                            connect_clicked => FileSetFormMsg::CancelDownload,
                        },
                    },
                },

                #[name = "selected_file_label"]
                gtk::Label {
                    #[watch]
                    set_label: &format!("Selected file: {}", model.picked_files
                    .iter()
                    .map(|f| f.path.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(", ")),
                },

               gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_min_content_height: 360,
                    set_vexpand: true,

                    #[local_ref]
                    files_list_box -> gtk::ListBox {}
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "File Set File Name:",
                    },

                    gtk::Entry {
                        set_placeholder_text: Some("File Set File Name"),
                        #[watch]
                        set_text: &model.file_set_file_name,
                        connect_activate[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(
                                FileSetFormMsg::FileSetFileNameChanged(buffer.text().into()),
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
                    gtk::Entry {
                        set_placeholder_text: Some("File Set Display Name"),
                        #[watch]
                        set_text: &model.file_set_name,
                        connect_activate[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(
                                FileSetFormMsg::FileSetNameChanged(buffer.text().into()),
                            );
                        }
                    },
                },

                gtk::Entry {
                    set_placeholder_text: Some("Source (e.g. website URL)"),
                    #[watch]
                    set_text: &model.source,
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            FileSetFormMsg::SourceChanged(buffer.text().into()),
                        );
                    }
                },

               gtk::Button {
                    #[watch]
                    set_label: if model.file_set_id.is_some() {
                        "Edit File Set"
                    } else {
                        "Create File Set"
                    },
                    connect_clicked => FileSetFormMsg::CreateOrUpdateFileSet,
                    #[watch]
                    set_sensitive: !model.selected_files_in_picked_files.is_empty() && !model.processing,
                },
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let files =
            FactoryVecDeque::builder()
                .launch_default()
                .forward(sender.input_sender(), |output| match output {
                    FileOutput::SetFileSelected {
                        sha1_checksum,
                        selected,
                    } => FileSetFormMsg::SetFileSelected {
                        sha1_checksum,
                        selected,
                    },
                });

        let dropdown = Self::create_dropdown(None, &sender);

        let file_import_service = Arc::new(FileImportService::new(
            Arc::clone(&init_model.repository_manager),
            Arc::clone(&init_model.settings),
        ));

        let download_service = Arc::new(DownloadService::new(
            Arc::clone(&init_model.repository_manager),
            Arc::clone(&init_model.settings),
        ));

        let item_type_dropdown = ItemTypeDropdown::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                ItemTypeDropDownOutputMsg::ItemTypeChanged(opt_item_type) => {
                    FileSetFormMsg::ItemTypeChanged(opt_item_type)
                }
            },
        );

        let model = FileSetFormModel {
            files,
            selected_system_ids: Vec::new(),
            file_set_name: String::new(),
            file_set_file_name: String::new(),
            source: String::new(),
            dropdown,
            processing: false,
            file_import_service,
            download_service,
            view_model_service: init_model.view_model_service,
            settings: Arc::clone(&init_model.settings),
            selected_file_type: None,
            selected_files_in_picked_files: Vec::new(),
            picked_files: Vec::new(),
            download_in_progress: false,
            download_total_size: None,
            download_bytes: 0,
            download_cancel_tx: None,
            file_set_id: None,
            item_type_dropdown,
            selected_item_type: None,
        };

        let file_types_dropdown = model.dropdown.widget();

        let files_list_box = model.files.widget();
        let item_type_dropdown = model.item_type_dropdown.widget();

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSetFormMsg::OpenFileSelector => {
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
                            sender.input(FileSetFormMsg::FileSelected(path));
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }

            FileSetFormMsg::FileSelected(path) => {
                if let Some(file_type) = self.selected_file_type {
                    let prepare_file_import_service = Arc::clone(&self.file_import_service);
                    self.processing = true;
                    sender.oneshot_command(async move {
                        let res = prepare_file_import_service
                            .prepare_import(&path, file_type)
                            .await;
                        CommandMsg::FileImportPrepared(res)
                    });
                }
            }

            FileSetFormMsg::OpenDownloadDialog => {
                let dialog = gtk::Dialog::builder()
                    .title("Download from URL")
                    .modal(true)
                    .transient_for(root)
                    .default_width(500)
                    .build();

                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Download", gtk::ResponseType::Accept);

                let content_area = dialog.content_area();
                let entry = gtk::Entry::builder()
                    .placeholder_text("Enter URL (e.g., https://example.com/file.zip)")
                    .margin_start(10)
                    .margin_end(10)
                    .margin_top(10)
                    .margin_bottom(10)
                    .build();

                content_area.append(&entry);

                dialog.connect_response(clone!(
                    #[strong]
                    sender,
                    #[strong]
                    entry,
                    move |dialog, response| {
                        if response == gtk::ResponseType::Accept {
                            let url = entry.text().to_string();
                            if !url.is_empty() {
                                sender.input(FileSetFormMsg::DownloadFromUrl(url));
                            }
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }

            FileSetFormMsg::DownloadFromUrl(url) => {
                if let Some(file_type) = self.selected_file_type {
                    let download_service = Arc::clone(&self.download_service);
                    let temp_dir = self.settings.temp_output_dir.clone();
                    self.source = url.clone();
                    self.processing = true;

                    // unbounded channel for download progress events
                    let (progress_tx, progress_rx) = unbounded::<HttpDownloadEvent>();

                    // Create cancellation channel
                    let (cancel_tx, cancel_rx) = unbounded::<()>();
                    self.download_cancel_tx = Some(cancel_tx);

                    // Spawn task to forward progress messages to UI
                    let ui_sender = sender.input_sender().clone();
                    task::spawn(async move {
                        while let Ok(event) = progress_rx.recv().await {
                            if let Err(e) =
                                ui_sender.send(FileSetFormMsg::ProcessDownloadEvent(event))
                            {
                                tracing::error!(
                                    error = ?e,
                                    "Error sending download event to UI, stopping handling events",
                                );
                                break;
                            }
                        }
                    });

                    sender.oneshot_command(async move {
                        let res = download_service
                            .download_and_prepare_import(
                                &url,
                                file_type,
                                &temp_dir,
                                progress_tx,
                                cancel_rx,
                            )
                            .await;
                        CommandMsg::FileImportPrepared(res)
                    });
                }
            }

            FileSetFormMsg::SetFileSelected {
                sha1_checksum,
                selected,
            } => {
                if selected && !self.selected_files_in_picked_files.contains(&sha1_checksum) {
                    self.selected_files_in_picked_files.push(sha1_checksum);
                } else if !selected && self.selected_files_in_picked_files.contains(&sha1_checksum)
                {
                    self.selected_files_in_picked_files
                        .retain(|s| *s != sha1_checksum);
                }
            }
            FileSetFormMsg::CreateOrUpdateFileSet => {
                if !self.selected_files_in_picked_files.is_empty()
                    && !self.processing
                    && let Some(file_type) = self.selected_file_type
                {
                    self.processing = true;

                    if let Some(file_set_id) = self.file_set_id {
                        self.update_file_set(sender, file_type, file_set_id);
                    } else {
                        self.create_file_set(sender, file_type);
                    }
                }
            }
            FileSetFormMsg::FileSetNameChanged(name) => {
                self.file_set_name = name;
            }
            FileSetFormMsg::FileSetFileNameChanged(name) => {
                self.file_set_file_name = name;
            }
            FileSetFormMsg::SourceChanged(source) => {
                self.source = source;
            }
            FileSetFormMsg::FileTypeChanged(file_type) => {
                self.selected_file_type = Some(file_type);
            }
            FileSetFormMsg::Show {
                selected_system_ids,
                selected_file_type,
            } => {
                self.picked_files.clear();
                self.selected_file_type = Some(selected_file_type);
                self.selected_system_ids = selected_system_ids;
                self.file_set_name.clear();
                self.file_set_file_name.clear();
                self.source.clear();
                self.files.guard().clear();
                self.dropdown
                    .emit(DropDownMsg::SetSelected(selected_file_type));
                root.show();
            }
            FileSetFormMsg::ShowEdit { file_set_id } => {
                tracing::info!(file_set_id = file_set_id, "Showing file set for editing");
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let res = view_model_service
                        .get_file_set_view_model(file_set_id)
                        .await;
                    CommandMsg::ProcessFileSetResponse(res)
                });
                root.show();
            }
            FileSetFormMsg::Hide => {
                root.hide();
            }
            FileSetFormMsg::CancelDownload => {
                if let Some(cancel_tx) = self.download_cancel_tx.take() {
                    // Send cancellation signal
                    if let Err(e) = cancel_tx.try_send(()) {
                        tracing::error!(error = ?e, "Failed to send cancel signal");
                    }
                }
            }
            FileSetFormMsg::ProcessDownloadEvent(event) => match event {
                HttpDownloadEvent::Started { total_size } => {
                    self.download_in_progress = true;
                    self.download_total_size = total_size;
                    self.download_bytes = 0;
                    tracing::info!(total_size = total_size, "Download started");
                }
                HttpDownloadEvent::Progress { bytes_downloaded } => {
                    self.download_bytes = bytes_downloaded;
                }
                HttpDownloadEvent::Completed { file_path } => {
                    self.download_in_progress = false;
                    self.download_bytes = 0;
                    self.download_total_size = None;
                    self.download_cancel_tx = None;
                    tracing::info!(file_path = ?file_path, "Download completed");
                }
                HttpDownloadEvent::Failed { error } => {
                    self.download_in_progress = false;
                    self.download_bytes = 0;
                    self.download_total_size = None;
                    self.download_cancel_tx = None;
                    tracing::error!(error = ?error, "Download failed");
                }
            },
            FileSetFormMsg::ItemTypeChanged(opt_item_type) => {
                tracing::info!("Item type changed (new component): {:?}", opt_item_type);
                self.selected_item_type = opt_item_type;
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::FileImportPrepared(Ok(prepare_result)) => {
                self.processing = false;
                let import_model = prepare_result.import_model;
                let import_metadata = prepare_result.import_metadata;
                for file in import_model.content.values() {
                    self.files.guard().push_back(ReadFile {
                        file_name: file.file_name.clone(),
                        sha1_checksum: file.sha1_checksum,
                        file_size: file.file_size,
                    });
                    // pre-select all files initially
                    self.selected_files_in_picked_files.push(file.sha1_checksum);
                }

                if self.file_set_name.is_empty() {
                    self.file_set_name = import_metadata.file_set_name.clone();
                }
                if self.file_set_file_name.is_empty() {
                    self.file_set_file_name = import_metadata.file_set_file_name.clone();
                }
                self.picked_files.push(import_model);
            }
            CommandMsg::ProcessCreateOrUpdateFileSetResult(Ok(import_result)) => {
                self.processing = false;
                if let Some(file_type) = self.selected_file_type {
                    let file_set_list_model = FileSetListModel {
                        id: import_result.file_set_id,
                        file_set_name: self.file_set_name.clone(),
                        file_type,
                        file_name: self.file_set_file_name.clone(),
                        can_delete: true,
                    };

                    let message = if self.file_set_id.is_some() {
                        FileSetFormOutputMsg::FileSetUpdated(file_set_list_model.clone())
                    } else {
                        FileSetFormOutputMsg::FileSetCreated(file_set_list_model.clone())
                    };

                    sender.output(message).unwrap_or_else(|err| {
                        tracing::error!(
                                error = ?err,
                                "Error sending output message");
                    });

                    root.close();
                }
            }
            CommandMsg::ProcessCreateOrUpdateFileSetResult(Err(e)) => {
                self.processing = false;
                tracing::error!(error = ?e, "File set import failed");
                show_error_dialog(format!("File set import failed: {:?}", e), root);
            }
            CommandMsg::FileImportPrepared(Err(e)) => {
                self.processing = false;
                tracing::error!(error = ?e, "Preparing file import failed");
                show_error_dialog(format!("Preparing file import failed: {:?}", e), root);
            }
            CommandMsg::ProcessFileSetResponse(Ok(file_set_view_model)) => {
                tracing::info!(
                    file_set_id = file_set_view_model.id,
                    "Loaded file set for editing",
                );
                self.file_set_id = Some(file_set_view_model.id);
                self.selected_file_type = Some(file_set_view_model.file_type);
                self.dropdown
                    .emit(DropDownMsg::SetSelected(file_set_view_model.file_type));
                // TODO: set system ids - why system ids are not included in FileSetViewModel?
                // Maybe they should be? Then there could be file sets without releases? Is that
                // needed?
                self.file_set_name = file_set_view_model.file_set_name.clone();
                self.file_set_file_name = file_set_view_model.file_name.clone();
                self.source = file_set_view_model.source.clone();
                // TODO: support multiple item types
                self.selected_item_type = file_set_view_model.item_types.first().cloned();
                tracing::info!(
                    "Setting selected item type in item type dropdown: {:?}",
                    self.selected_item_type
                );
                self.item_type_dropdown
                    .emit(ItemTypeDropDownMsg::SetSelectedItemType(
                        self.selected_item_type,
                    ));
                self.files.guard().clear();
                for file in file_set_view_model.files.iter() {
                    self.files.guard().push_back(ReadFile {
                        file_name: file.file_name.clone(),
                        sha1_checksum: file.sha1_checksum,
                        file_size: file.file_size,
                    });
                    // pre-select all files initially
                    self.selected_files_in_picked_files.push(file.sha1_checksum);
                }
            }
            CommandMsg::ProcessFileSetResponse(Err(e)) => {
                tracing::error!(error = ?e, "Failed to load file set for editing");
                show_error_dialog(
                    format!("Failed to load file set for editing: {:?}", e),
                    root,
                );
            }
        }
    }
}

impl FileSetFormModel {
    fn create_file_set(&self, sender: ComponentSender<Self>, file_type: FileType) {
        tracing::info!("Creating new file set");
        let item_types = if let Some(item_type) = self.selected_item_type {
            vec![item_type]
        } else {
            vec![]
        };
        let file_import_model = FileSetImportModel {
            file_set_name: self.file_set_name.clone(),
            file_set_file_name: self.file_set_file_name.clone(),
            source: self.source.clone(),
            file_type,
            system_ids: self.selected_system_ids.clone(),
            selected_files: self.selected_files_in_picked_files.clone(),
            import_files: self.picked_files.clone(),
            item_ids: vec![],
            item_types,
        };

        let file_import_service = Arc::clone(&self.file_import_service);

        sender.oneshot_command(async move {
            let import_result = file_import_service.create_file_set(file_import_model).await;
            CommandMsg::ProcessCreateOrUpdateFileSetResult(import_result)
        });
    }

    fn update_file_set(
        &self,
        sender: ComponentSender<Self>,
        file_type: FileType,
        file_set_id: i64,
    ) {
        tracing::info!(file_set_id = file_set_id, "Updating file set");
        let item_types = if let Some(item_type) = self.selected_item_type {
            vec![item_type]
        } else {
            vec![]
        };
        let update_model = UpdateFileSetModel {
            file_set_name: self.file_set_name.clone(),
            file_set_file_name: self.file_set_file_name.clone(),
            source: self.source.clone(),
            file_type,
            selected_files: self.selected_files_in_picked_files.clone(),
            import_files: self.picked_files.clone(),
            file_set_id,
            item_ids: vec![],
            item_types,
        };

        let file_import_service = Arc::clone(&self.file_import_service);

        sender.oneshot_command(async move {
            let import_result = file_import_service.update_file_set(update_model).await;
            CommandMsg::ProcessCreateOrUpdateFileSetResult(import_result)
        });
    }
}
