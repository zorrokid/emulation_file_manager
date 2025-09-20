use std::sync::Arc;

use core_types::FileType;
use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};
use ui_components::{DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::{
    file_set_form::{FileSetFormInit, FileSetFormModel, FileSetFormMsg, FileSetFormOutputMsg},
    list_item::ListItem,
};

#[derive(Debug)]
pub enum FileSelectMsg {
    FetchFiles,
    SelectClicked,
    OpenFileSetForm,
    FileSetCreated(FileSetListModel),
    FileSetSelected {
        index: u32,
    },
    FileTypeChanged(FileType),
    Show {
        selected_system_ids: Vec<i64>,
        selected_file_set_ids: Vec<i64>,
    },
    Hide,
    Ignore,
}

#[derive(Debug)]
pub enum FileSelectOutputMsg {
    FileSetSelected(FileSetListModel),
    //FileSetUpdated(FileSetListModel),
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
}

#[derive(Debug)]
pub struct FileSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    file_sets: Vec<FileSetListModel>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    file_set_form: Controller<FileSetFormModel>,
    selected_system_ids: Vec<i64>,
    selected_file_type: Option<FileType>,
    selected_file_set: Option<FileSetListModel>,
    selected_file_set_ids: Vec<i64>,
    dropdown: Controller<FileTypeDropDown>,
}

#[relm4::component(pub)]
impl Component for FileSelectModel {
    type Input = FileSelectMsg;
    type Output = FileSelectOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSelectInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 800,

            connect_close_request[sender] => move |_| {
                sender.input(FileSelectMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::Label {
                    set_label: "File Selector",
                },
                #[local_ref]
                file_types_dropdown -> gtk::Box {},
                gtk::Button {
                    set_label: "Add File Set",
                    connect_clicked => FileSelectMsg::OpenFileSetForm,
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    file_set_list_view -> gtk::ListView {}
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
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> = TypedListView::new();

        let dropdown = FileTypeDropDown::builder().launch(None).forward(
            sender.input_sender(),
            |msg| match msg {
                DropDownOutputMsg::ItemSelected(FileTypeSelectedMsg::FileTypeSelected(
                    file_type,
                )) => FileSelectMsg::FileTypeChanged(file_type),
                _ => unreachable!(),
            },
        );

        let file_set_form_init_model = FileSetFormInit {
            repository_manager: Arc::clone(&init_model.repository_manager),
            settings: Arc::clone(&init_model.settings),
            // TODO set in Show message
            //selected_system_ids: self.selected_system_ids.clone(),
            //selected_file_type,
        };
        let file_set_form = FileSetFormModel::builder()
            .transient_for(&root)
            .launch(file_set_form_init_model)
            .forward(sender.input_sender(), |msg| match msg {
                FileSetFormOutputMsg::FileSetCreated(file_set_liset_model) => {
                    FileSelectMsg::FileSetCreated(file_set_liset_model)
                }
                _ => FileSelectMsg::Ignore,
            });

        let model = FileSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            file_sets: Vec::new(),
            list_view_wrapper,
            file_set_form,
            selected_system_ids: Vec::new(),
            selected_file_type: None,
            selected_file_set: None,
            selected_file_set_ids: Vec::new(),
            dropdown,
        };
        let file_types_dropdown = model.dropdown.widget();
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
                    self.file_set_form.emit(FileSetFormMsg::Show {
                        selected_system_ids: self.selected_system_ids.clone(),
                        selected_file_type,
                    });
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
                        file_set_name: selected_item.name.clone(), // TODO
                        file_type: file_type,
                        file_name: selected_item.name.clone(),
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
                        file_type: file_type,
                        file_name: file_set.name.clone(), // TODO?
                    };
                    self.selected_file_set = Some(file_set_list_model);
                } else {
                    eprintln!("No file set found at index {}", index);
                }
            }
            FileSelectMsg::FileTypeChanged(file_type) => {
                println!("File type changed to: {:?}", file_type);
                self.selected_file_type = Some(file_type);
                sender.input(FileSelectMsg::FetchFiles);
            }
            FileSelectMsg::FetchFiles => {
                println!("Fetching file sets for selected systems and file type");
                if let Some(file_type) = self.selected_file_type {
                    println!("Selected file type: {:?}", file_type);
                    let view_model_service = Arc::clone(&self.view_model_service);
                    let system_ids = self.selected_system_ids.clone();
                    println!("Selected system IDs: {:?}", system_ids);
                    sender.oneshot_command(clone!(
                        #[strong]
                        view_model_service,
                        async move {
                            let file_sets = view_model_service
                                .get_file_set_list_models(file_type, &system_ids)
                                .await;
                            CommandMsg::FilesFetched(file_sets)
                        }
                    ));
                }
            }
            FileSelectMsg::Show {
                selected_system_ids,
                selected_file_set_ids,
            } => {
                self.selected_system_ids = selected_system_ids;
                self.selected_file_set_ids = selected_file_set_ids;
                sender.input(FileSelectMsg::FetchFiles);
                root.show();
            }
            FileSelectMsg::Hide => {
                root.hide();
            }
            _ => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CommandMsg::FilesFetched(Ok(file_sets)) => {
                println!("File sets fetched successfully: {:?}", file_sets);
                self.file_sets = file_sets;
                self.list_view_wrapper.clear();
                let list_items = self
                    .file_sets
                    .iter()
                    .filter(|f| !self.selected_file_set_ids.contains(&f.id))
                    .map(|file_set| ListItem {
                        id: file_set.id,
                        name: format!("{} [{}]", file_set.file_set_name, file_set.file_name),
                    });
                self.list_view_wrapper.extend_from_iter(list_items);
            }
            CommandMsg::FilesFetched(Err(e)) => {
                eprintln!("Failed to fetch file sets: {:?}", e);
                // TODO handle error
            }
            _ => {
                // Handle command outputs here
            }
        }
    }
}
