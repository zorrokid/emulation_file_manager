use std::{collections::HashMap, path::PathBuf, sync::Arc};

use core_types::{FileType, ReadFile, Sha1Checksum};
use database::{
    database_error::Error as DatabaseError, models::FileInfo, repository_manager::RepositoryManager,
};
use file_import::FileImportError;
use relm4::{
    Component, ComponentParts, ComponentSender, FactorySender, RelmWidgetExt,
    gtk::{
        self, FileChooserDialog,
        gio::prelude::FileExt,
        glib::clone,
        prelude::{
            BoxExt, ButtonExt, CheckButtonExt, DialogExt, FileChooserExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
    prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque},
};
use service::{view_model_service::ViewModelService, view_models::FileSetListModel};

use crate::file_importer::FileImporter;
use strum::IntoEnumIterator;

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
                set_active: false,
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
            selected: false,
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

/*#[derive(Debug, Clone)]
struct DropDownFileType {
    name: String,
    file_type: FileType,
    selected: bool,
}

#[derive(Debug)]
enum DropDownFileTypeInput {
    FileTypeSelected { file_type: FileType },
}

#[derive(Debug)]
enum DropDownFileTypeOutput {
    FileTypeSelected { file_type: FileType },
}

#[relm4::factory]
impl FactoryComponent for DropDownFileType {
    type Init = FileType;
    type Input = DropDownFileTypeInput;
    type Output = DropDownFileTypeOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::DropDown;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            #[name(label)]
            gtk::Label {
                set_label: &self.name,
                set_hexpand: true,
                set_halign: gtk::Align::Start,
                set_margin_all: 12,
            },
        }
    }

    fn init_model(
        file_type: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self {
            name: file_type.to_string(),
            file_type,
            selected: false,
        }
    }

    fn update(&mut self, message: Self::Input, _sender: FactorySender<Self>) {
        match message {
            _ => {
                // Handle input messages here
            }
        }
    }
}*/

//

#[derive(Debug)]
pub enum FileSetFormMsg {
    OpenFileSelector,
    FileSelected(PathBuf),
    CreateFileSetFromSelectedFiles,
    SetFileSelected {
        sha1_checksum: Sha1Checksum,
        selected: bool,
    },
    SetFileTypeSelected {
        index: u32,
    },
}

#[derive(Debug)]
pub enum FileSetFormOutputMsg {
    FileSetCreated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FileSelected,
    FileContentsRead(Result<HashMap<Sha1Checksum, ReadFile>, FileImportError>),
    ExistingFilesRead(Result<Vec<FileInfo>, DatabaseError>),
}

pub struct FileSetFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub struct FileSetFormModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    file_importer: FileImporter,
    files: FactoryVecDeque<File>,
    file_types: Vec<FileType>,
    selected_file_type: Option<FileType>,
}

/*#[derive(Debug)]
struct Widgets {
    pub selected_file_label: gtk::Label,
}*/

#[relm4::component(pub)]
impl Component for FileSetFormModel {
    type Input = FileSetFormMsg;
    type Output = FileSetFormOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSetFormInit;
    //type Widgets = Widgets;
    //type Root = gtk::Window;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,

                gtk::Button {
                    set_label: "Open file selector",
                    connect_clicked => FileSetFormMsg::OpenFileSelector,
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

                #[local_ref]
                file_types_dropdown -> gtk::DropDown {
                    connect_selected_notify[sender] => move |dropdown| {
                        sender.input(FileSetFormMsg::SetFileTypeSelected {
                            index: dropdown.selected(),
                        });
                    }

                },

                gtk::Button {
                    set_label: "Create File Set",
                    connect_clicked => FileSetFormMsg::CreateFileSetFromSelectedFiles,
                    #[watch]
                    set_sensitive: model.selected_file_type.is_some() && model.file_importer.is_selected_files(),
                },
            }
        }
    }
    /*fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Create file set")
            .default_width(800)
            .default_height(800)
            .build()
    }*/

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        /*let open_file_selector_button = gtk::Button::with_label("Open file selector");
        open_file_selector_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(FileSetFormMsg::OpenFileSelector);
            }
        ));
        let v_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();
        v_box.append(&open_file_selector_button);
        root.set_child(Some(&v_box));

        let model = FileSetFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            file_importer: FileImporter::new(),
        };
        let selected_file_label = gtk::Label::new(Some("No file selected"));
        v_box.append(&selected_file_label);
        let widgets = Widgets {
            selected_file_label,
        };*/

        let files = FactoryVecDeque::builder()
            .launch_default()
            //.detach();
            .forward(sender.input_sender(), |output| match output {
                FileOutput::SetFileSelected {
                    sha1_checksum,
                    selected,
                } => FileSetFormMsg::SetFileSelected {
                    sha1_checksum,
                    selected,
                },
            });

        /*let file_types = FactoryVecDeque::builder()
        .launch_default()
        //.detach();
        .forward(sender.input_sender(), |output| match output {
            DropDownFileTypeOutput::FileTypeSelected { file_type } => {
                FileSetFormMsg::SetFileTypeSelected { file_type }
            }
        });*/

        /*file_types.append(
            FileType::iter()
                .map(|file_type| DropDownFileType::new(file_type))
                .collect::<Vec<_>>(),
        );*/

        let file_types: Vec<FileType> = FileType::iter().collect();

        let file_types_dropdown = gtk::DropDown::builder().build();
        let file_types_to_drop_down: Vec<String> =
            file_types.iter().map(|ft| ft.to_string()).collect();
        let file_types_str: Vec<&str> =
            file_types_to_drop_down.iter().map(|s| s.as_str()).collect();

        let file_types_drop_down_model = gtk::StringList::new(&file_types_str);

        file_types_dropdown.set_model(Some(&file_types_drop_down_model));

        let model = FileSetFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            file_importer: FileImporter::new(),
            files,
            file_types,
            selected_file_type: None,
        };
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
                        if response == gtk::ResponseType::Accept {
                            if let Some(path) = dialog.file().and_then(|f| f.path()) {
                                sender.input(FileSetFormMsg::FileSelected(path));
                            }
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }
            FileSetFormMsg::FileSelected(path) => {
                println!("File selected: {:?}", path);
                self.file_importer.set_current_picked_file(path.clone());
                let is_zip_file = self.file_importer.is_zip_file();
                sender.oneshot_command(async move {
                    // TODO: combine this in file_import
                    let res = match is_zip_file {
                        true => file_import::read_zip_contents_with_checksums(path),
                        false => file_import::read_file_checksum(path),
                    };
                    CommandMsg::FileContentsRead(res)
                });
            }
            FileSetFormMsg::CreateFileSetFromSelectedFiles => {
                self.files.guard().iter().for_each(|file| {
                    println!("File {} selected: {}", file.name, file.selected);
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
            FileSetFormMsg::SetFileTypeSelected { index } => {
                println!("File type selected from index: {}", index);
                let file_type = self
                    .file_types
                    .get(index as usize)
                    .cloned()
                    .expect("Invalid file type index");
                println!("Selected file type: {:?}", file_type);
                self.selected_file_type = Some(file_type);
            }
            _ => {

                // Handle input messages here
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            CommandMsg::FileContentsRead(Ok(file_contents)) => {
                println!("File contents read successfully: {:?}", file_contents);
                let file_checksums = file_contents.keys().cloned().collect::<Vec<Sha1Checksum>>();
                self.file_importer
                    .set_current_picked_file_content(file_contents);

                let repository_manager = Arc::clone(&self.repository_manager);
                sender.oneshot_command(async move {
                    let existing_files_file_info = repository_manager
                        .get_file_info_repository()
                        .get_file_infos_by_sha1_checksums(file_checksums)
                        .await;
                    CommandMsg::ExistingFilesRead(existing_files_file_info)
                });
            }
            CommandMsg::FileContentsRead(Err(e)) => {
                eprintln!("Error reading file contents: {:?}", e);
                // TODO: show error to user
            }
            CommandMsg::ExistingFilesRead(Ok(existing_files_file_info)) => {
                println!(
                    "Existing files read successfully: {:?}",
                    existing_files_file_info
                );
                self.file_importer
                    .set_existing_files(existing_files_file_info);

                for file in self
                    .file_importer
                    .get_current_picked_file_content()
                    .values()
                {
                    self.files.guard().push_back(file.clone());
                }
            }
            CommandMsg::ExistingFilesRead(Err(e)) => {
                eprintln!("Error reading existing files: {:?}", e);
                // TODO: show error to user
            }
            CommandMsg::FileSelected => {
                // This is a placeholder for handling file selection command output
                // You can update the UI or perform other actions here
                println!("File selected command executed");
            }
            _ => {
                // Handle command outputs here
            }
        }
    }

    /*fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let selected_file_text = self
            .file_importer
            .get_current_picked_file()
            .map_or("No file selected".to_string(), |path| {
                path.to_string_lossy().to_string()
            });
        widgets
            .selected_file_label
            .set_text(selected_file_text.as_str());
    }*/
}
