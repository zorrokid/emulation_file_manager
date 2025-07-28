use std::sync::Arc;

use core_types::FileType;
use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, SelectionModelExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError,
    view_model_service::ViewModelService,
    view_models::{FileSetListModel, Settings},
};

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
    seleccted_file_type: Option<FileType>,
}

#[derive(Debug)]
pub struct Widgets {}

impl Component for FileSelectModel {
    type Input = FileSelectMsg;
    type Output = FileSelectOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = FileSelectInit;
    type Widgets = Widgets;
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("File Selector")
            .default_width(800)
            .default_height(800)
            .build()
    }
    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let v_box = gtk::Box::builder()
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
        v_box.append(&add_file_set_button);
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> = TypedListView::new();
        let files_list = &list_view_wrapper.view;
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
        root.set_child(Some(&v_box));

        let widgets = Widgets {};
        let model = FileSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            file_sets: Vec::new(),
            list_view_wrapper,
            file_set_form: None,
            selected_system_ids: init_model.selected_system_ids,
            seleccted_file_type: None,
        };
        sender.input(FileSelectMsg::FetchFiles);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            FileSelectMsg::OpenFileSetForm => {
                let init_model = FileSetFormInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                    settings: Arc::clone(&self.settings),
                    selected_system_ids: self.selected_system_ids.clone(),
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
            FileSelectMsg::FileSetCreated(file_set_list_model) => {
                println!("File set created {}", file_set_list_model);
                self.list_view_wrapper.append(ListItem {
                    id: file_set_list_model.id,
                    name: file_set_list_model.file_set_name.clone(),
                });
            }
            FileSelectMsg::SelectClicked => {
                let selection = self.list_view_wrapper.selection_model.selected();
                if let (Some(selected_item), Some(file_type)) = (
                    self.list_view_wrapper.get(selection),
                    self.seleccted_file_type,
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
