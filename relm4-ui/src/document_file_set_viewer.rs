use std::sync::Arc;

use crate::{
    document_viewer_form::{
        DocumentViewerFormInit, DocumentViewerFormModel, DocumentViewerFormMsg,
        DocumentViewerFormOutputMsg,
    },
    list_item::ListItem,
};
use database::{
    database_error::DatabaseError, models::FileSetFileInfo, repository_manager::RepositoryManager,
};
use emulator_runner::{error::EmulatorRunnerError, run_with_emulator};
use file_export::export_files;
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
    export_service::prepare_fileset_for_export,
    file_set_download::service::{DownloadResult, DownloadService},
    settings_service::SettingsService,
    view_model_service::ViewModelService,
    view_models::{DocumentViewerListModel, DocumentViewerViewModel, FileSetViewModel, Settings},
};
use ui_components::confirm_dialog::{
    ConfirmDialog, ConfirmDialogInit, ConfirmDialogMsg, ConfirmDialogOutputMsg,
};

#[derive(Debug)]
pub enum DocumentViewerMsg {
    FetchViewers,

    // list selection messages
    FileSelected { index: u32 },
    ViewerSelected { index: u32 },

    OpenViewerForm,
    AddViewer(DocumentViewerListModel),
    UpdateViewer(DocumentViewerListModel),

    PrepareFilesForViewer,
    StartViewer,
    StartEdit,
    ConfirmDelete,
    DeleteConfirmed,

    Show { file_set: FileSetViewModel },
    Hide,

    Ignore,
}

#[derive(Debug)]
pub enum DocumentViewerCommandMsg {
    ViewersFetched(Result<Vec<DocumentViewerViewModel>, ServiceError>),
    FinishedRunningViewer(Result<(), EmulatorRunnerError>),
    Deleted(Result<i64, DatabaseError>),
    FilePreparationDone(Result<DownloadResult, ServiceError>),
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
    file_download_service: Arc<DownloadService>,
    settings_service: Arc<SettingsService>,

    // list views
    file_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    viewer_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,

    // controllers
    viewer_form: Controller<DocumentViewerFormModel>,
    confirm_dialog_controller: Controller<ConfirmDialog>,

    // data
    viewers: Vec<DocumentViewerViewModel>,

    // needed for running the viewer:
    settings: Arc<Settings>,
    file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfo>,
    selected_viewer: Option<DocumentViewerViewModel>,
}

#[relm4::component(pub)]
impl Component for DocumentViewer {
    type Input = DocumentViewerMsg;
    type Output = ();
    type CommandOutput = DocumentViewerCommandMsg;
    type Init = DocumentViewerInit;

    view! {
        gtk::Window {
            set_title: Some("Document Viewer"),

            connect_close_request[sender] => move |_| {
                sender.input(DocumentViewerMsg::Hide);
                glib::Propagation::Stop
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                file_list_view -> gtk::ListView,

                #[local_ref]
                viewer_list_view -> gtk::ListView,

                gtk::Button {
                    set_label: "Add",
                    connect_clicked => DocumentViewerMsg::OpenViewerForm,
                },

                gtk::Button {
                    set_label: "Edit",
                    connect_clicked => DocumentViewerMsg::StartEdit,
                    #[watch]
                    set_sensitive: model.selected_viewer.is_some()
                },

                gtk::Button {
                    set_label: "Delete",
                    connect_clicked => DocumentViewerMsg::ConfirmDelete,
                    #[watch]
                    set_sensitive: model.selected_viewer.is_some()
                },


                gtk::Button {
                    set_label: "Start",
                    connect_clicked => DocumentViewerMsg::PrepareFilesForViewer,
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

        let init_model = DocumentViewerFormInit {
            repository_manager: Arc::clone(&init.repository_manager),
        };
        let viewer_form = DocumentViewerFormModel::builder()
            .transient_for(&root)
            .launch(init_model)
            .forward(sender.input_sender(), |msg| match msg {
                DocumentViewerFormOutputMsg::DocumentViewerAdded(viewer_list_model) => {
                    DocumentViewerMsg::AddViewer(viewer_list_model)
                }
                DocumentViewerFormOutputMsg::DocumentViewerUpdated(viewer_list_model) => {
                    println!("Viewer updated: {:?}", viewer_list_model);
                    DocumentViewerMsg::UpdateViewer(viewer_list_model)
                }
            });

        let confirm_dialog_controller = ConfirmDialog::builder()
            .transient_for(&root)
            .launch(ConfirmDialogInit {
                title: "Confirm Deletion".to_string(),
                visible: false,
            })
            .forward(sender.input_sender(), |msg| match msg {
                ConfirmDialogOutputMsg::Confirmed => DocumentViewerMsg::DeleteConfirmed,
                ConfirmDialogOutputMsg::Canceled => DocumentViewerMsg::Ignore,
            });
        let settings_service = Arc::new(SettingsService::new(Arc::clone(&init.repository_manager)));

        let file_download_service = Arc::new(DownloadService::new(
            Arc::clone(&init.repository_manager),
            Arc::clone(&init.settings),
            Arc::clone(&settings_service),
        ));

        let model = DocumentViewer {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,
            file_download_service,
            settings_service,

            viewers: Vec::new(),
            settings: init.settings,
            file_set: None,

            file_list_view_wrapper,
            viewer_list_view_wrapper,

            selected_file: None,
            selected_viewer: None,
            viewer_form,
            confirm_dialog_controller,
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
            DocumentViewerMsg::PrepareFilesForViewer => {
                if let Some(file_set) = &self.file_set {
                    let download_service = Arc::clone(&self.file_download_service);
                    let file_set_id = file_set.id;

                    sender.oneshot_command(async move {
                        let res = download_service
                            .download_file_set(file_set_id, true, None)
                            .await;
                        DocumentViewerCommandMsg::FilePreparationDone(res)
                    });
                }
            }
            DocumentViewerMsg::StartViewer => {
                if let (Some(viewer), Some(selected_file), Some(file_set)) =
                    (&self.selected_viewer, &self.selected_file, &self.file_set)
                {
                    let executable = viewer.executable.clone();
                    // TODO: create a viewer view model that has processed arguments already to
                    // correct format
                    let arguments = Vec::new(); // TODO: viewer.arguments.clone();
                    let files_in_fileset = file_set
                        .files
                        .iter()
                        .map(|f| f.file_name.clone())
                        .collect::<Vec<_>>();

                    let starting_file = selected_file.file_name.clone();
                    let temp_dir = self.settings.temp_output_dir.clone();

                    sender.oneshot_command(async move {
                        let res = run_with_emulator(
                            executable,
                            &arguments,
                            &files_in_fileset,
                            starting_file,
                            temp_dir,
                        )
                        .await;
                        DocumentViewerCommandMsg::FinishedRunningViewer(res)
                    });
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
                self.viewer_form.emit(DocumentViewerFormMsg::Show {
                    edit_document_viewer: None,
                });
            }
            DocumentViewerMsg::AddViewer(_viewer_list_model) => {
                sender.input(DocumentViewerMsg::FetchViewers);
            }
            DocumentViewerMsg::UpdateViewer(_viewer_list_model) => {
                println!("Viewer updated: {:?}", _viewer_list_model);
                sender.input(DocumentViewerMsg::FetchViewers);
            }
            DocumentViewerMsg::FetchViewers => {
                println!("Fetching viewers");

                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let viewers_result = view_model_service.get_document_viewer_view_models().await;
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
            DocumentViewerMsg::StartEdit => {
                if let Some(selected_viewer) = self.selected_viewer.clone() {
                    self.viewer_form.emit(DocumentViewerFormMsg::Show {
                        edit_document_viewer: Some(selected_viewer),
                    });
                }
            }
            DocumentViewerMsg::ConfirmDelete => {
                self.confirm_dialog_controller.emit(ConfirmDialogMsg::Show);
            }
            DocumentViewerMsg::DeleteConfirmed => {
                if let Some(selected_viewer) = &self.selected_viewer {
                    let viewer_id = selected_viewer.id;
                    let repository_manager = Arc::clone(&self.repository_manager);
                    sender.oneshot_command(async move {
                        let res = repository_manager
                            .get_document_viewer_repository()
                            .delete(viewer_id)
                            .await;
                        DocumentViewerCommandMsg::Deleted(res)
                    });
                }
            }

            _ => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            DocumentViewerCommandMsg::ViewersFetched(Ok(viewer_view_models)) => {
                println!("Viewers fetched successfully: {:?}", viewer_view_models);
                let viewer_list_items = viewer_view_models
                    .iter()
                    .map(|viewer| ListItem {
                        id: viewer.id,
                        name: viewer.name.clone(),
                    })
                    .collect::<Vec<_>>();
                self.viewers = viewer_view_models;
                self.viewer_list_view_wrapper.clear();
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
            DocumentViewerCommandMsg::Deleted(Ok(_)) => {
                println!("Viewer deleted successfully");
                // TODO: delete directly from the list?
                sender.input(DocumentViewerMsg::FetchViewers);
            }
            DocumentViewerCommandMsg::Deleted(Err(error)) => {
                eprintln!("Error deleting viewer: {:?}", error);
            }
            DocumentViewerCommandMsg::FilePreparationDone(Ok(_download_result)) => {
                println!("Files prepared successfully for viewer");
                sender.input(DocumentViewerMsg::StartViewer);
            }
            DocumentViewerCommandMsg::FilePreparationDone(Err(error)) => {
                eprintln!("Error preparing files for viewer: {:?}", error);
            }
        }
    }
}
