use std::sync::Arc;

use core_types::FileType;
use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, SelectionModelExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};
use strum::IntoEnumIterator;

use crate::{
    file_set_form::{FileSetFormInit, FileSetFormModel, FileSetFormOutputMsg},
    list_item::ListItem,
};

#[derive(Debug)]
pub enum FileSelectMsg {
    FetchFiles,
    AddFileSet,
    SelectClicked,
    OpenFileSetForm,
    FileSetCreated(FileSetListModel),
    FileSetSelected { index: u32 },
    SetFileTypeSelected { index: u32 },
}

#[derive(Debug)]
pub enum FileSelectOutputMsg {
    FileSetSelected(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FilesFetched(Result<Vec<FileSetListModel>, ServiceError>),
    FileSetAdded(FileSetListModel),
    AddingFileSetFailed(DatabaseError),
}

pub struct FileSelectInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    pub selected_system_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct FileSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    file_sets: Vec<FileSetListModel>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    file_set_form: Option<Controller<FileSetFormModel>>,
    selected_system_ids: Vec<i64>,
    selected_file_type: Option<FileType>,
    selected_file_set: Option<FileSetListModel>,
    file_types: Vec<FileType>,
}

#[derive(Debug)]
pub struct Widgets {}

#[relm4::component(pub)]
impl Component for FileSelectModel {
    type Input = FileSelectMsg;
    type Output = FileSelectOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSelectInit;
    //type Widgets = Widgets;
    //type Root = gtk::Window;

    /*fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("File Selector")
            .default_width(800)
            .default_height(800)
            .build()
    }*/

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 800,
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Label {
                    set_label: "File Selector Component",
                },
                #[local_ref]
                file_types_dropdown -> gtk::DropDown {
                    connect_selected_notify[sender] => move |dropdown| {
                        sender.input(FileSelectMsg::SetFileTypeSelected {
                            index: dropdown.selected(),
                        });
                    }

                },


                gtk::Button {
                    set_label: "Add File Set",
                    connect_clicked => FileSelectMsg::OpenFileSetForm,
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    file_set_list_view -> gtk::ListView {

                    }
                },

                gtk::Button {
                    set_label: "Select File Set",
                    connect_clicked => FileSelectMsg::SelectClicked,
                    #[watch]
                    set_sensitive: model.selected_file_set.is_some() && model.selected_file_type.is_some(),
                },
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        /*let v_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let add_file_set_button = gtk::Button::with_label("Add File Set");
        add_file_set_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(FileSelectMsg::OpenFileSetForm);
            }
        ));
        v_box.append(&add_file_set_button);*/
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> = TypedListView::new();
        /*let files_list = &list_view_wrapper.view;
        let files_list_container = gtk::ScrolledWindow::builder().vexpand(true).build();
        files_list_container.set_child(Some(files_list));

        let label = gtk::Label::new(Some("Select file set"));
        v_box.append(&label);
        v_box.append(&files_list_container);
        let select_button = gtk::Button::with_label("Select File Set");
        select_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(FileSelectMsg::SelectClicked);
            }
        ));
        v_box.append(&select_button);
        root.set_child(Some(&v_box));*/

        //let widgets = Widgets {};
        let file_types: Vec<FileType> = FileType::iter().collect();

        let file_types_dropdown = gtk::DropDown::builder().build();
        let file_types_to_drop_down: Vec<String> =
            file_types.iter().map(|ft| ft.to_string()).collect();
        let file_types_str: Vec<&str> =
            file_types_to_drop_down.iter().map(|s| s.as_str()).collect();

        let file_types_drop_down_model = gtk::StringList::new(&file_types_str);

        file_types_dropdown.set_model(Some(&file_types_drop_down_model));

        let model = FileSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            file_sets: Vec::new(),
            list_view_wrapper,
            file_set_form: None,
            selected_system_ids: init_model.selected_system_ids,
            selected_file_type: None,
            selected_file_set: None,
            file_types,
        };
        let file_set_list_view = &model.list_view_wrapper.view;
        model
            .list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    println!("File Select - Selected item index: {:?}", selected);
                    sender.input(FileSelectMsg::FileSetSelected { index: selected });
                }
            ));
        let widgets = view_output!();
        sender.input(FileSelectMsg::FetchFiles);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSelectMsg::OpenFileSetForm => {
                if let Some(selected_file_type) = self.selected_file_type {
                    let init_model = FileSetFormInit {
                        view_model_service: Arc::clone(&self.view_model_service),
                        repository_manager: Arc::clone(&self.repository_manager),
                        settings: Arc::clone(&self.settings),
                        selected_system_ids: self.selected_system_ids.clone(),
                        selected_file_type,
                    };
                    let file_set_form = FileSetFormModel::builder().launch(init_model).forward(
                        sender.input_sender(),
                        |msg| match msg {
                            FileSetFormOutputMsg::FileSetCreated(file_set_liset_model) => {
                                FileSelectMsg::FileSetCreated(file_set_liset_model)
                            }
                        },
                    );
                    self.file_set_form = Some(file_set_form);

                    self.file_set_form
                        .as_ref()
                        .expect("File set form should be set")
                        .widget()
                        .present();
                }
            }
            FileSelectMsg::FileSetCreated(file_set_list_model) => {
                println!("File Selector - File set created {}", file_set_list_model);
                self.list_view_wrapper.append(ListItem {
                    id: file_set_list_model.id,
                    name: file_set_list_model.file_set_name.clone(),
                });
            }
            FileSelectMsg::SelectClicked => {
                let selection = self.list_view_wrapper.selection_model.selected();
                println!("File Select Clicked - Selected item: {:?}", selection);
                if let (Some(selected_item), Some(file_type)) = (
                    self.list_view_wrapper.get(selection),
                    self.selected_file_type,
                ) {
                    let selected_item = selected_item.borrow();
                    println!(
                        "File set selected: {} with ID: {}",
                        selected_item.name, selected_item.id
                    );
                    let file_set_list_model = FileSetListModel {
                        id: selected_item.id,
                        file_set_name: selected_item.name.clone(),
                        file_type: file_type.into(),
                    };
                    let res =
                        sender.output(FileSelectOutputMsg::FileSetSelected(file_set_list_model));
                    if let Err(e) = res {
                        eprintln!("Failed to send output message: {:?}", e);
                        // TODO handle error
                    } else {
                        println!("File set selection output sent successfully.");
                        root.close();
                    }
                } else {
                    eprintln!("No file set selected");
                }
            }
            FileSelectMsg::FileSetSelected { index } => {
                println!("File set selected at index: {}", index);
                if let (Some(file_set), Some(file_type)) =
                    (self.list_view_wrapper.get(index), self.selected_file_type)
                {
                    let file_set = file_set.borrow();
                    println!(
                        "File set selected: {} with ID: {}",
                        file_set.name, file_set.id
                    );
                    let file_set_list_model = FileSetListModel {
                        id: file_set.id,
                        file_set_name: file_set.name.clone(),
                        file_type: file_type.into(),
                    };
                    self.selected_file_set = Some(file_set_list_model);
                } else {
                    eprintln!("No file set found at index {}", index);
                }
            }
            FileSelectMsg::SetFileTypeSelected { index } => {
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
                // Handle other messages here
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
            _ => {
                // Handle command outputs here
            }
        }
    }
}
