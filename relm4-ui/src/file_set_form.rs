use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use core_types::{FileType, ImportedFile, ReadFile, Sha1Checksum};
use database::{
    database_error::Error as DatabaseError, models::FileInfo, repository_manager::RepositoryManager,
};
use file_import::{FileImportError, FileImportModel};
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
use service::view_models::{FileSetListModel, Settings};
use ui_components::{DropDownMsg, DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::file_importer::FileImporter;

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
    FileContentsRead(
        PathBuf,
        Result<HashMap<Sha1Checksum, ReadFile>, FileImportError>,
    ),
    ExistingFilesRead {
        path: PathBuf,
        content: HashMap<Sha1Checksum, ReadFile>,
        result: Result<Vec<FileInfo>, DatabaseError>,
    },
    FilesImported(Result<HashMap<Sha1Checksum, ImportedFile>, FileImportError>),
    FilesSavedToDatabase(Result<i64, DatabaseError>),
}

pub struct FileSetFormInit {
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct FileSetFormModel {
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    file_importer: FileImporter,
    files: FactoryVecDeque<File>,
    selected_system_ids: Vec<i64>,
    file_set_name: String,
    file_set_file_name: String,
    source: String,
    dropdown: Controller<FileTypeDropDown>,
    processing: bool,
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
                    set_sensitive: model.file_importer.get_selected_file_type().is_some(),
                },

                #[name = "selected_file_label"]
                gtk::Label {
                    #[watch]
                    set_label: &format!("Selected file: {}", model.file_importer),
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
                    set_sensitive: model.file_importer.is_selected_files() && !model.processing,
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

        let model = FileSetFormModel {
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            file_importer: FileImporter::new(),
            files,
            selected_system_ids: Vec::new(),
            file_set_name: String::new(),
            file_set_file_name: String::new(),
            source: String::new(),
            dropdown,
            processing: false,
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
                println!("File selected: {:?}", path);
                let file_set_name = self.file_importer.get_file_set_name(&path);
                let file_set_file_name = self.file_importer.get_file_set_file_name(&path);
                if self.file_set_name.is_empty() {
                    self.file_set_name = file_set_name.clone().unwrap_or_default();
                }
                if self.file_set_file_name.is_empty() {
                    self.file_set_file_name = file_set_file_name.unwrap_or_default();
                }
                let is_zip_file = self.file_importer.is_zip_file(&path);
                sender.oneshot_command(async move {
                    let res = match is_zip_file {
                        true => file_import::read_zip_contents_with_checksums(&path),
                        false => file_import::read_file_checksum(&path),
                    };
                    CommandMsg::FileContentsRead(path, res)
                });
            }
            FileSetFormMsg::SetFileSelected {
                sha1_checksum,
                selected,
            } => {
                println!(
                    "File with checksum {:?} selected: {}",
                    sha1_checksum, selected
                );
                if selected {
                    self.file_importer.select_file(&sha1_checksum);
                } else {
                    self.file_importer.deselect_file(&sha1_checksum);
                }
            }
            FileSetFormMsg::CreateFileSetFromSelectedFiles => {
                if self.file_importer.is_selected_files() && !self.processing {
                    self.processing = true;
                    let file_type = self.file_importer.get_selected_file_type().expect(
                        "File type must be selected (should have been checked in beginning of process)",
                    );
                    let target_path = self.settings.get_file_type_path(&file_type);
                    let new_files_file_name_filter = self
                        .file_importer
                        .get_selected_files_from_current_picked_files_that_are_new()
                        .iter()
                        .map(|file| file.file_name.clone())
                        .collect::<HashSet<String>>();

                    let file_import_model = FileImportModel {
                        file_path: self
                            .file_importer
                            .get_current_picked_files()
                            .iter()
                            .map(|f| f.path.clone())
                            .collect::<Vec<PathBuf>>(),
                        output_dir: target_path.to_path_buf(),
                        file_name: self.file_set_file_name.clone(),
                        file_set_name: self.file_set_name.clone(),
                        file_type,
                        new_files_file_name_filter,
                    };

                    println!("Prepared file import model: {:?}", file_import_model);
                    sender.oneshot_command(async move {
                        let res = file_import::import(&file_import_model);
                        CommandMsg::FilesImported(res)
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
                println!("File type changed: {:?}", file_type);
                self.file_importer.set_selected_file_type(file_type);
            }
            FileSetFormMsg::Show {
                selected_system_ids,
                selected_file_type,
            } => {
                self.file_importer.clear();
                self.file_importer
                    .set_selected_file_type(selected_file_type);
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
            CommandMsg::FileContentsRead(path, Ok(content)) => {
                println!("File contents read successfully: {:?}", &content);
                let file_checksums = content.keys().cloned().collect::<Vec<Sha1Checksum>>();
                let repository_manager = Arc::clone(&self.repository_manager);
                let file_type = self.file_importer.get_selected_file_type().expect(
                    "File type must be selected (should have been checked in beginning of process)",
                );
                sender.oneshot_command(async move {
                    let existing_files_file_info = repository_manager
                        .get_file_info_repository()
                        .get_file_infos_by_sha1_checksums(file_checksums, file_type)
                        .await;
                    CommandMsg::ExistingFilesRead {
                        path,
                        content,
                        result: existing_files_file_info,
                    }
                });
            }
            CommandMsg::FileContentsRead(_, Err(e)) => {
                eprintln!("Error reading file contents: {:?}", e);
                // TODO: show error to user
            }
            CommandMsg::ExistingFilesRead {
                path,
                content,
                result,
            } => match result {
                Ok(existing_files_file_info) => {
                    println!(
                        "Existing files read successfully: {:?}",
                        existing_files_file_info
                    );
                    for file in content.values() {
                        self.files.guard().push_back(file.clone());
                    }
                    self.file_importer.add_new_picked_file(
                        &path,
                        &content,
                        &existing_files_file_info,
                    );
                }
                Err(e) => {
                    eprintln!("Error reading existing files: {:?}", e);
                }
            },
            CommandMsg::FilesImported(Ok(imported_files_map)) => {
                self.processing = false;
                println!("Files imported successfully: {:?}", imported_files_map);
                self.file_importer
                    .set_imported_files(imported_files_map.clone());

                let system_ids = self.selected_system_ids.clone();
                let repo = Arc::clone(&self.repository_manager);

                let files_in_file_set = self.file_importer.get_files_selected_for_file_set();
                let file_set_name = self.file_set_name.clone();
                let file_set_file_name = self.file_set_file_name.clone();
                let source = self.source.clone();
                let file_type = self.file_importer.get_selected_file_type().expect(
                    "File type must be selected (should have been checked in beginning of process)",
                );
                assert!(!files_in_file_set.is_empty());
                assert!(!file_set_file_name.is_empty());
                sender.oneshot_command(async move {
                    let result = repo
                        .get_file_set_repository()
                        .add_file_set(
                            &file_set_name,
                            &file_set_file_name,
                            &file_type,
                            &source,
                            &files_in_file_set,
                            &system_ids,
                        )
                        .await;
                    CommandMsg::FilesSavedToDatabase(result)
                });
            }
            CommandMsg::FilesImported(Err(e)) => {
                self.processing = false;
                eprintln!("Error importing files: {:?}", e);
                // TODO: show error to user
            }
            CommandMsg::FilesSavedToDatabase(Ok(id)) => {
                println!("Files saved to database successfully with ID: {}", id);
                let file_type = self.file_importer.get_selected_file_type().expect(
                    "File type must be selected (should have been checked in beginning of process)",
                );
                let file_set_list_model = FileSetListModel {
                    id,
                    file_set_name: self.file_set_name.clone(),
                    file_type,
                    file_name: self.file_set_file_name.clone(),
                };
                let res = sender.output(FileSetFormOutputMsg::FileSetCreated(file_set_list_model));
                if let Err(e) = res {
                    eprintln!("Error sending output: {:?}", e);
                    // TODO: show error to user
                } else {
                    println!("File set created successfully");
                    root.close();
                }
            }
            CommandMsg::FilesSavedToDatabase(Err(e)) => {
                eprintln!("Error saving files to database: {:?}", e);
            }
            _ => {}
        }
    }
}
