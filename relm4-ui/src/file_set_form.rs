use std::{path::PathBuf, sync::Arc};

use async_std::{channel::unbounded, task};
use core_types::{FileType, ReadFile, Sha1Checksum, events::HttpDownloadEvent};
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
        model::{FileImportModel, FileImportPrepareResult, FileSetImportModel, ImportFileContent},
        service::FileImportService,
    },
    view_models::{FileSetListModel, Settings},
};
use ui_components::{DropDownMsg, DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::utils::{dialog_utils::show_message_dialog, string_utils::format_bytes};

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
                    let res = sender.output(FileOutput::SetFileSelected {
                        sha1_checksum,
                        selected: checkbox.is_active(),
                    });
                    if let Err(e) = res {
                        eprintln!("Error sending output: {:?}", e);
                    }
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
    CreateFileSetFromSelectedFiles,
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
    Hide,
    ProcessDownloadEvent(HttpDownloadEvent),
    CancelDownload,
}

#[derive(Debug)]
pub enum FileSetFormOutputMsg {
    FileSetCreated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FileImportPrepared(Result<FileImportPrepareResult, Error>),
    FileImportDone(Result<i64, Error>),
}

pub struct FileSetFormInit {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct FileSetFormModel {
    files: FactoryVecDeque<File>,
    selected_system_ids: Vec<i64>,
    file_set_name: String,
    file_set_file_name: String,
    source: String,
    dropdown: Controller<FileTypeDropDown>,
    processing: bool,
    file_import_service: Arc<FileImportService>,
    download_service: Arc<DownloadService>,
    settings: Arc<Settings>,
    selected_file_type: Option<FileType>,
    selected_files_in_picked_files: Vec<Sha1Checksum>,
    picked_files: Vec<FileImportModel>,
    // Download progress tracking
    download_in_progress: bool,
    download_total_size: Option<u64>,
    download_bytes: u64,
    download_cancel_tx: Option<async_std::channel::Sender<()>>,
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
            set_title: Some("Create File Set"),

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
                    set_label: "Create File Set",
                    connect_clicked => FileSetFormMsg::CreateFileSetFromSelectedFiles,
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
            settings: Arc::clone(&init_model.settings),
            selected_file_type: None,
            selected_files_in_picked_files: Vec::new(),
            picked_files: Vec::new(),
            download_in_progress: false,
            download_total_size: None,
            download_bytes: 0,
            download_cancel_tx: None,
        };
        let file_types_dropdown = model.dropdown.widget();

        let files_list_box = model.files.widget();

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSetFormMsg::OpenFileSelector => {
                println!("Open file selector button clicked");
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
                                eprintln!(
                                    "Error sending download event to UI, stopping handling events: {:?}",
                                    e
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
            FileSetFormMsg::CreateFileSetFromSelectedFiles => {
                if !self.selected_files_in_picked_files.is_empty()
                    && !self.processing
                    && let Some(file_type) = self.selected_file_type
                {
                    self.processing = true;

                    let import_files: Vec<FileImportModel> = self
                        .picked_files
                        .iter()
                        .map(|f| FileImportModel {
                            path: f.path.clone(),
                            content: f
                                .content
                                .iter()
                                .map(|(k, v)| {
                                    let existing_file_info_id = v.existing_file_info_id;
                                    let existing_archive_file_name =
                                        v.existing_archive_file_name.clone();
                                    (
                                        *k,
                                        ImportFileContent {
                                            file_name: v.file_name.clone(),
                                            sha1_checksum: *k,
                                            file_size: v.file_size,
                                            existing_file_info_id,
                                            existing_archive_file_name,
                                        },
                                    )
                                })
                                .collect(),
                        })
                        .collect();

                    let file_import_model = FileSetImportModel {
                        file_set_name: self.file_set_name.clone(),
                        file_set_file_name: self.file_set_file_name.clone(),
                        source: self.source.clone(),
                        file_type,
                        system_ids: self.selected_system_ids.clone(),
                        selected_files: self.selected_files_in_picked_files.clone(),
                        import_files,
                    };

                    let file_import_service = Arc::clone(&self.file_import_service);

                    sender.oneshot_command(async move {
                        let import_result = file_import_service.import(file_import_model).await;
                        CommandMsg::FileImportDone(import_result)
                    });
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
            FileSetFormMsg::Hide => {
                root.hide();
            }
            FileSetFormMsg::CancelDownload => {
                if let Some(cancel_tx) = self.download_cancel_tx.take() {
                    // Send cancellation signal
                    if let Err(e) = cancel_tx.try_send(()) {
                        eprintln!("Failed to send cancel signal: {:?}", e);
                    }
                }
            }
            FileSetFormMsg::ProcessDownloadEvent(event) => match event {
                HttpDownloadEvent::Started { total_size } => {
                    self.download_in_progress = true;
                    self.download_total_size = total_size;
                    self.download_bytes = 0;
                    println!("Download started (size: {:?})", total_size);
                }
                HttpDownloadEvent::Progress { bytes_downloaded } => {
                    self.download_bytes = bytes_downloaded;
                }
                HttpDownloadEvent::Completed { file_path } => {
                    self.download_in_progress = false;
                    self.download_bytes = 0;
                    self.download_total_size = None;
                    self.download_cancel_tx = None;
                    println!("Download completed: {:?}", file_path);
                }
                HttpDownloadEvent::Failed { error } => {
                    self.download_in_progress = false;
                    self.download_bytes = 0;
                    self.download_total_size = None;
                    self.download_cancel_tx = None;
                    eprintln!("Download failed: {}", error);
                }
            },
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
                let import_file = prepare_result.import_model;
                let import_metadata = prepare_result.import_metadata;
                for file in import_file.content.values() {
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
                self.picked_files.push(import_file);
            }
            CommandMsg::FileImportDone(Ok(id)) => {
                self.processing = false;
                if let Some(file_type) = self.selected_file_type {
                    let file_set_list_model = FileSetListModel {
                        id,
                        file_set_name: self.file_set_name.clone(),
                        file_type,
                        file_name: self.file_set_file_name.clone(),
                        can_delete: true,
                    };

                    let res =
                        sender.output(FileSetFormOutputMsg::FileSetCreated(file_set_list_model));

                    if let Err(e) = res {
                        eprintln!("Error sending output: {:?}", e);
                        // TODO: show error to user
                    } else {
                        println!("File set created successfully");
                        root.close();
                    }
                }
            }
            CommandMsg::FileImportDone(Err(e)) => {
                self.processing = false;
                eprintln!("Error importing file set: {:?}", e);
            }
            CommandMsg::FileImportPrepared(Err(e)) => {
                self.processing = false;
                eprintln!("Error preparing file import: {:?}", e);
                show_message_dialog(
                    format!("Preparing file import failed: {:?}", e),
                    gtk::MessageType::Error,
                    root,
                );
            }
        }
    }
}
