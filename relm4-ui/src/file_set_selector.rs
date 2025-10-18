use std::sync::Arc;

use core_types::FileType;
use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    file_set_deletion_service::FileSetDeletionService,
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};
use ui_components::{DropDownOutputMsg, FileTypeDropDown, FileTypeSelectedMsg};

use crate::{
    file_set_details_view::{FileSetDetailsInit, FileSetDetailsMsg, FileSetDetailsView},
    file_set_form::{FileSetFormInit, FileSetFormModel, FileSetFormMsg, FileSetFormOutputMsg},
    list_item::ListItem,
};

#[derive(Debug)]
pub enum FileSetSelectorMsg {
    FetchFiles,
    SelectClicked,
    DeleteClicked,
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
pub enum FileSetSelectorOutputMsg {
    FileSetSelected(FileSetListModel),
    //FileSetUpdated(FileSetListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    FilesFetched(Result<Vec<FileSetListModel>, ServiceError>),
    FileSetAdded(FileSetListModel),
    AddingFileSetFailed(DatabaseError),
    FilesSetDeletionFinished(Result<(), ServiceError>),
}

pub struct FileSetSelectorInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct FileSetSelector {
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
    file_set_details_view: Controller<FileSetDetailsView>,
    file_set_deletion_service: Arc<FileSetDeletionService>,
}

#[relm4::component(pub)]
impl Component for FileSetSelector {
    type Input = FileSetSelectorMsg;
    type Output = FileSetSelectorOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSetSelectorInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 800,
            set_title: Some("Select File Set"),

            connect_close_request[sender] => move |_| {
                sender.input(FileSetSelectorMsg::Hide);
                glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                gtk::Paned {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_start_child: Some(&main_box),
                    set_end_child: Some(&file_set_details),
                },

                #[name = "main_box"]
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "File Selector",
                    },
                    #[local_ref]
                    file_types_dropdown -> gtk::Box {},
                    gtk::Button {
                        set_label: "Add File Set",
                        connect_clicked => FileSetSelectorMsg::OpenFileSetForm,
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[local_ref]
                        file_set_list_view -> gtk::ListView {}
                    },

                    gtk::Button {
                        set_label: "Select File Set",
                        connect_clicked => FileSetSelectorMsg::SelectClicked,
                        #[watch]
                        set_sensitive: model.selected_file_set.is_some() && model.selected_file_type.is_some(),
                    },
                    gtk::Button {
                        set_label: "Delete File Set",
                        connect_clicked => FileSetSelectorMsg::DeleteClicked,
                        #[watch]
                        set_sensitive: model.selected_file_set.is_some(),
                    },
                    gtk::Label {
                        set_label: "When deleting a file set, also that actual files will be deleted unless they are linked to other file sets (in that case only those files that are linked won't be deleted).",
                    },
                },

                #[name = "file_set_details"]
                gtk::Box {
                    #[local_ref]
                    file_set_details_view -> gtk::Box {},
                },
            },
        },
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
                )) => FileSetSelectorMsg::FileTypeChanged(file_type),
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
                    FileSetSelectorMsg::FileSetCreated(file_set_liset_model)
                }
                _ => FileSetSelectorMsg::Ignore,
            });

        let file_set_details_view_init = FileSetDetailsInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
        };

        // TODO: is this needed to be Controller?
        let file_set_details_view = FileSetDetailsView::builder()
            .launch(file_set_details_view_init)
            .forward(sender.input_sender(), |_| FileSetSelectorMsg::Ignore);

        let file_set_deletion_service = Arc::new(FileSetDeletionService::new(
            Arc::clone(&init_model.repository_manager),
            Arc::clone(&init_model.settings),
        ));

        let model = FileSetSelector {
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
            file_set_details_view,
            file_set_deletion_service,
        };
        let file_types_dropdown = model.dropdown.widget();
        let file_set_list_view = &model.list_view_wrapper.view;
        let file_set_details_view = model.file_set_details_view.widget();
        model
            .list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    println!("File Select - Selected item index: {:?}", selected);
                    sender.input(FileSetSelectorMsg::FileSetSelected { index: selected });
                }
            ));
        let widgets = view_output!();
        sender.input(FileSetSelectorMsg::FetchFiles);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSetSelectorMsg::OpenFileSetForm => {
                if let Some(selected_file_type) = self.selected_file_type {
                    self.file_set_form.emit(FileSetFormMsg::Show {
                        selected_system_ids: self.selected_system_ids.clone(),
                        selected_file_type,
                    });
                }
            }
            FileSetSelectorMsg::FileSetCreated(file_set_list_model) => {
                println!("File Selector - File set created {}", file_set_list_model);
                self.list_view_wrapper.append(ListItem {
                    id: file_set_list_model.id,
                    name: file_set_list_model.file_set_name.clone(),
                });
            }
            FileSetSelectorMsg::SelectClicked => {
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
                        file_type,
                        file_name: selected_item.name.clone(),
                    };
                    let res = sender.output(FileSetSelectorOutputMsg::FileSetSelected(
                        file_set_list_model,
                    ));
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
            FileSetSelectorMsg::FileSetSelected { index } => {
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
                        file_type,
                        file_name: file_set.name.clone(), // TODO?
                    };
                    self.selected_file_set = Some(file_set_list_model);
                    self.file_set_details_view
                        .emit(FileSetDetailsMsg::LoadFileSet(file_set.id));
                } else {
                    eprintln!("No file set found at index {}", index);
                }
            }
            FileSetSelectorMsg::FileTypeChanged(file_type) => {
                println!("File type changed to: {:?}", file_type);
                self.selected_file_type = Some(file_type);
                sender.input(FileSetSelectorMsg::FetchFiles);
            }
            FileSetSelectorMsg::FetchFiles => {
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
            FileSetSelectorMsg::Show {
                selected_system_ids,
                selected_file_set_ids,
            } => {
                self.selected_system_ids = selected_system_ids;
                self.selected_file_set_ids = selected_file_set_ids;
                sender.input(FileSetSelectorMsg::FetchFiles);
                root.show();
            }
            FileSetSelectorMsg::Hide => {
                root.hide();
            }
            FileSetSelectorMsg::DeleteClicked => {
                if let Some(selected_file_set) = &self.selected_file_set {
                    let file_set_deletion_service = self.file_set_deletion_service.clone();
                    let file_set_id = selected_file_set.id;

                    sender.oneshot_command(clone!(
                        #[strong]
                        file_set_deletion_service,
                        async move {
                            let res = file_set_deletion_service.delete_file_set(file_set_id).await;
                            // Return a command message if needed
                            CommandMsg::FilesSetDeletionFinished(res)
                        }
                    ));
                }
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
            CommandMsg::FilesSetDeletionFinished(Ok(())) => {
                println!("File set deleted successfully.");
                // Refresh the file sets list
            }
            CommandMsg::FilesSetDeletionFinished(Err(e)) => {
                eprintln!("Failed to delete file set: {:?}", e);
                // TODO handle error
            }
            _ => {
                // Handle command outputs here
            }
        }
    }
}
