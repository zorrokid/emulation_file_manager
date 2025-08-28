use std::sync::Arc;

use crate::{
    document_viewer_form::{
        DocumentViewerFormInit, DocumentViewerFormModel, DocumentViewerFormOutputMsg,
    },
    list_item::ListItem,
    utils::prepare_fileset_for_export,
};
use database::{
    database_error::DatabaseError,
    models::{DocumentViewer as DocumentViewerDbModel, FileSetFileInfo},
    repository_manager::RepositoryManager,
};
use emulator_runner::{error::EmulatorRunnerError, run_with_emulator};
use file_export::export_files;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{DocumentViewerListModel, FileSetViewModel, Settings},
};

#[derive(Debug)]
pub enum DocumentViewerMsg {
    FetchViewers,

    // list selection messages
    FileSelected { index: u32 },
    ViewerSelected { index: u32 },

    OpenViewerForm,
    AddViewer(DocumentViewerListModel),

    StartViewer,

    Show { file_set: FileSetViewModel },
    Hide,
}

#[derive(Debug)]
pub enum DocumentViewerCommandMsg {
    ViewersFetched(Result<Vec<DocumentViewerDbModel>, DatabaseError>),
    FinishedRunningViewer(Result<(), EmulatorRunnerError>),
}

pub struct DocumentViewerInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct DocumentViewer {
    // services
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,

    // list views
    file_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    viewer_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,

    // controllers
    viewer_form: Option<Controller<DocumentViewerFormModel>>,

    // data
    viewers: Vec<DocumentViewerDbModel>,

    // needed for running the viewer:
    settings: Arc<Settings>,
    file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfo>,
    selected_viewer: Option<DocumentViewerDbModel>,
}

#[relm4::component(pub)]
impl Component for DocumentViewer {
    type Input = DocumentViewerMsg;
    type Output = ();
    type CommandOutput = DocumentViewerCommandMsg;
    type Init = DocumentViewerInit;

    view! {
        gtk::Window {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                file_list_view -> gtk::ListView,

                #[local_ref]
                viewer_list_view -> gtk::ListView,

                gtk::Button {
                    set_label: "Add viewer",
                    connect_clicked => DocumentViewerMsg::OpenViewerForm,

                },

                gtk::Button {
                    set_label: "Start viewer",
                    connect_clicked => DocumentViewerMsg::StartViewer,
                    #[watch]
                    set_sensitive: model.selected_viewer.is_some() && model.selected_file.is_some(),
                },
            }
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let file_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();
        let viewer_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();

        let model = DocumentViewer {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,

            viewers: Vec::new(),
            settings: init.settings,
            file_set: None,

            file_list_view_wrapper,
            viewer_list_view_wrapper,

            selected_file: None,
            selected_viewer: None,
            viewer_form: None,
        };

        let file_list_view = &model.file_list_view_wrapper.view;
        let viewer_list_view = &model.viewer_list_view_wrapper.view;

        model
            .file_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(DocumentViewerMsg::FileSelected { index: selected });
                }
            ));

        model
            .viewer_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(DocumentViewerMsg::ViewerSelected { index: selected });
                }
            ));

        let widgets = view_output!();
        sender.input(DocumentViewerMsg::FileSelected { index: 0 });
        sender.input(DocumentViewerMsg::FetchViewers);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            DocumentViewerMsg::StartViewer => {
                println!("Starting viewer...");
                if let (Some(viewer), Some(selected_file), Some(file_set)) = (
                    self.selected_viewer.clone(),
                    self.selected_file.clone(),
                    self.file_set.clone(),
                ) {
                    let temp_dir = std::env::temp_dir();
                    let export_model = prepare_fileset_for_export(
                        &file_set,
                        &self.settings.collection_root_dir,
                        temp_dir.as_path(),
                        true,
                    );

                    println!("Export model prepared: {:?}", export_model);

                    let files_in_fileset = file_set
                        .files
                        .iter()
                        .map(|f| f.file_name.clone())
                        .collect::<Vec<_>>();

                    let starting_file = selected_file.file_name.clone();

                    let executable = viewer.executable.clone();
                    // TODO: create a viewer view model that has processed arguments already to
                    // correct format
                    let arguments = Vec::new(); // TODO: viewer.arguments.clone();

                    sender.oneshot_command(async move {
                        let res = match export_files(&export_model) {
                            Ok(()) => {
                                run_with_emulator(
                                    executable,
                                    &arguments,
                                    &files_in_fileset,
                                    starting_file,
                                    temp_dir,
                                )
                                .await
                            }
                            Err(e) => Err(EmulatorRunnerError::IoError(format!(
                                "Failed to export files: {}",
                                e
                            ))),
                        };
                        DocumentViewerCommandMsg::FinishedRunningViewer(res)
                    });
                } else {
                    // Handle the case where no viewer or file is selected
                    eprintln!("No viewer or file selected");
                }
            }
            DocumentViewerMsg::FileSelected { index } => {
                println!("File selected at index: {}", index);
                let file_list_item = self.file_list_view_wrapper.get(index);
                if let (Some(item), Some(file_set)) = (file_list_item, &self.file_set) {
                    let id = item.borrow().id;
                    let file_info = file_set.files.iter().find(|f| f.file_info_id == id);
                    self.selected_file = file_info.cloned();
                }
            }
            DocumentViewerMsg::ViewerSelected { index } => {
                println!("Viewer selected at index: {}", index);
                let viewer_list_item = self.viewer_list_view_wrapper.get(index);
                if let Some(item) = viewer_list_item {
                    let id = item.borrow().id;
                    let viewer = self.viewers.iter().find(|e| e.id == id);
                    self.selected_viewer = viewer.cloned();
                }
            }
            DocumentViewerMsg::OpenViewerForm => {
                println!("Open Viewer Form");
                let init_model = DocumentViewerFormInit {
                    view_model_service: Arc::clone(&self.view_model_service),
                    repository_manager: Arc::clone(&self.repository_manager),
                };
                let viewer_form = DocumentViewerFormModel::builder()
                    .transient_for(root)
                    .launch(init_model)
                    .forward(sender.input_sender(), |msg| match msg {
                        DocumentViewerFormOutputMsg::DocumentViewerAdded(viewer_list_model) => {
                            DocumentViewerMsg::AddViewer(viewer_list_model)
                        }
                    });

                self.viewer_form = Some(viewer_form);
                self.viewer_form
                    .as_ref()
                    .expect("Viewer form should be initialized")
                    .widget()
                    .present();
            }
            DocumentViewerMsg::AddViewer(_viewer_list_model) => {
                sender.input(DocumentViewerMsg::FetchViewers);
            }
            DocumentViewerMsg::FetchViewers => {
                println!("Fetching viewers");
                let repository = Arc::clone(&self.repository_manager);
                sender.oneshot_command(async move {
                    let viewers_result = repository
                        .get_document_viewer_repository()
                        .get_document_viewers()
                        .await;
                    DocumentViewerCommandMsg::ViewersFetched(viewers_result)
                });
            }
            DocumentViewerMsg::Show { file_set } => {
                let file_list_items = file_set
                    .files
                    .iter()
                    .map(|file| ListItem {
                        id: file.file_info_id,
                        name: file.file_name.clone(),
                    })
                    .collect::<Vec<_>>();

                self.file_list_view_wrapper.clear();
                self.file_list_view_wrapper
                    .extend_from_iter(file_list_items);

                self.file_set = Some(file_set);
                root.show();
            }
            DocumentViewerMsg::Hide => {
                root.hide();
            }
            _ => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            DocumentViewerCommandMsg::ViewersFetched(Ok(viewer_db_models)) => {
                println!("Viewers fetched successfully: {:?}", viewer_db_models);
                let viewer_list_items = viewer_db_models
                    .iter()
                    .map(|viewer| ListItem {
                        id: viewer.id,
                        name: viewer.name.clone(),
                    })
                    .collect::<Vec<_>>();
                self.viewers = viewer_db_models;
                self.viewer_list_view_wrapper
                    .extend_from_iter(viewer_list_items);
            }
            DocumentViewerCommandMsg::ViewersFetched(Err(error)) => {
                eprintln!("Error fetching viewers: {:?}", error);
                // TODO: Handle error appropriately, e.g., show a dialog or log the error
            }
            DocumentViewerCommandMsg::FinishedRunningViewer(Ok(())) => {
                println!("Viewer ran successfully");
                root.close();
            }
            DocumentViewerCommandMsg::FinishedRunningViewer(Err(error)) => {
                eprintln!("Error running viewer: {:?}", error);
            }
        }
    }
}
