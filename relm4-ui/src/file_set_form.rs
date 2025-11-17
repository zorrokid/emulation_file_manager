use std::{collections::HashMap, path::PathBuf, sync::Arc};

use core_types::{FileType, ImportedFile, ReadFile, Sha1Checksum};
use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use file_import::FileImportError;
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
    error::Error,
    file_import::{
        model::{FileImportModel, FileSetImportModel, ImportFileContent},
        service::FileImportService,
    },
    view_models::{FileSetListModel, Settings},
};
use ui_components::{DropDownMsg, DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

/*
// TODO: move to prepare file import service?
#[derive(Debug)]
pub struct PickedFile {
    pub path: PathBuf,
    pub content: HashMap<Sha1Checksum, PickedFileContent>,
}


// TODO: move to prepare file import service?
#[derive(Debug)]
pub struct PickedFileContent {
    pub file_name: String,
    pub sha1_checksum: Sha1Checksum,
    pub file_size: FileSize,
    pub existing_archive_file_name: Option<String>,
    pub existing_file_info_id: Option<i64>,
}*/

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
}

#[derive(Debug)]
pub enum FileSetFormOutputMsg {
    FileSetCreated(FileSetListModel),
    FileSetUpdated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FilesImported(Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError>),
    FilesSavedToDatabase(Result<i64, DatabaseError>),
    FileImportPrepared(Result<FileImportModel, Error>),
    FileImportDone(Result<i64, Error>),
}

pub struct FileSetFormInit {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct FileSetFormModel {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    files: FactoryVecDeque<File>,
    selected_system_ids: Vec<i64>,
    file_set_name: String,
    file_set_file_name: String,
    source: String,
    dropdown: Controller<FileTypeDropDown>,
    processing: bool,
    file_import_service: Arc<FileImportService>,
    selected_file_type: Option<FileType>,
    selected_files_in_picked_files: Vec<Sha1Checksum>,
    picked_files: Vec<FileImportModel>,
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

                #[local_ref]
                file_types_dropdown -> gtk::Box,

                gtk::Button {
                    set_label: "Open file selector",
                    connect_clicked => FileSetFormMsg::OpenFileSelector,
                    #[watch]
                    set_sensitive: model.selected_file_type.is_some(),
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


                gtk::Entry {
                    set_placeholder_text: Some("File Set Description"),
                    #[watch]
                    set_text: &model.file_set_name,
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(
                            FileSetFormMsg::FileSetNameChanged(buffer.text().into()),
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

        let model = FileSetFormModel {
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            files,
            selected_system_ids: Vec::new(),
            file_set_name: String::new(),
            file_set_file_name: String::new(),
            source: String::new(),
            dropdown,
            processing: false,
            file_import_service,
            selected_file_type: None,
            selected_files_in_picked_files: Vec::new(),
            picked_files: Vec::new(),
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
                    sender.oneshot_command(async move {
                        let res = prepare_file_import_service
                            .prepare_import(&path, file_type)
                            .await;
                        CommandMsg::FileImportPrepared(res)
                    });
                } else {
                    eprintln!("File type not selected");
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
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::FileImportPrepared(Ok(import_file)) => {
                println!("File import prepared successfully: {:?}", import_file);
                for file in import_file.content.values() {
                    self.files.guard().push_back(ReadFile {
                        file_name: file.file_name.clone(),
                        sha1_checksum: file.sha1_checksum,
                        file_size: file.file_size,
                    });
                }

                // If PickedFile and PickedFileContent structures were in service crate,
                // prepare_file_import service could return PickedFile directly.
                // PickedFile /-Content could be renamed to ImportFile /-Content.
                /*let picked_file = PickedFile {
                    path: import_file.path,
                    content: import_file
                        .content
                        .into_iter()
                        .map(|(k, v)| {
                            (
                                k,
                                PickedFileContent {
                                    file_name: v.file_name.clone(),
                                    sha1_checksum: k,
                                    file_size: v.file_size,
                                    existing_archive_file_name: v
                                        .existing_archive_file_name
                                        .clone(),
                                    existing_file_info_id: v.existing_file_info_id,
                                },
                            )
                        })
                        .collect(),
                };
                self.picked_files.push(picked_file);*/

                // TODO: onko nämä tarpeen?
                /*if self.file_set_name.is_empty() {
                    self.file_set_name = import_file.file_set_name.clone();
                }
                if self.file_set_file_name.is_empty() {
                    self.file_set_file_name = import_file.file_set_file_name.clone();
                }*/
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
            _ => {}
        }
    }
}
