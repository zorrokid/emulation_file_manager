use std::sync::Arc;

use crate::{
    document_viewer_form::{
        DocumentViewerFormInit, DocumentViewerFormModel, DocumentViewerFormMsg,
        DocumentViewerFormOutputMsg,
    },
    list_item::ListItem,
    utils::dialog_utils::show_error_dialog,
};
use database::{database_error::DatabaseError, repository_manager::RepositoryManager};
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
    app_services::AppServices,
    error::Error as ServiceError,
    external_executable_runner::service::{ExecutableRunnerModel, ExternalExecutableRunnerService},
    view_model_service::ViewModelService,
    view_models::{
        DocumentViewerListModel, DocumentViewerViewModel, FileSetFileInfoViewModel,
        FileSetViewModel, Settings,
    },
};
use ui_components::confirm_dialog::{
    ConfirmDialog, ConfirmDialogInit, ConfirmDialogMsg, ConfirmDialogOutputMsg,
};

#[derive(Debug)]
pub enum DocumentViewerMsg {
    FetchViewers,

    // list selection messages
    FileSelected,
    ViewerSelected,

    OpenViewerForm,
    AddViewer(DocumentViewerListModel),
    UpdateViewer(DocumentViewerListModel),

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
    FinishedRunningViewer(Result<(), ServiceError>),
    Deleted(Result<i64, DatabaseError>),
}

pub struct DocumentViewerInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub app_services: Arc<AppServices>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub struct DocumentViewer {
    // services
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    app_services: Arc<AppServices>,
    external_executable_runner_service: Arc<ExternalExecutableRunnerService>,

    // list views
    file_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    viewer_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,

    // controllers
    viewer_form: Controller<DocumentViewerFormModel>,
    confirm_dialog_controller: Controller<ConfirmDialog>,

    // data
    viewers: Vec<DocumentViewerViewModel>,

    // needed for running the viewer:
    file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfoViewModel>,
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

        let init_model = DocumentViewerFormInit {
            repository_manager: Arc::clone(&init.repository_manager),
            app_services: Arc::clone(&init.app_services),
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

        let external_executable_runner_service = Arc::new(ExternalExecutableRunnerService::new(
            Arc::clone(&init.settings),
            Arc::clone(&init.repository_manager),
        ));

        let model = DocumentViewer {
            view_model_service: init.view_model_service,
            repository_manager: init.repository_manager,
            app_services: init.app_services,
            external_executable_runner_service,

            viewers: Vec::new(),
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
                move |_| {
                    sender.input(DocumentViewerMsg::FileSelected);
                }
            ));

        model
            .viewer_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |_| {
                    sender.input(DocumentViewerMsg::ViewerSelected);
                }
            ));

        let widgets = view_output!();
        sender.input(DocumentViewerMsg::FileSelected);
        sender.input(DocumentViewerMsg::FetchViewers);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            DocumentViewerMsg::StartViewer => {
                if let (Some(viewer), Some(selected_file), Some(file_set)) =
                    (&self.selected_viewer, &self.selected_file, &self.file_set)
                {
                    let executable = viewer.executable.clone();
                    // TODO: create a viewer view model that has processed arguments already to
                    // correct format
                    let arguments = Vec::new(); // TODO: viewer.arguments.clone();
                    let executable_runner_service =
                        Arc::clone(&self.external_executable_runner_service);

                    let executable_runner_model = ExecutableRunnerModel {
                        executable,
                        arguments,
                        extract_files: true,
                        file_set_id: file_set.id,
                        initial_file: Some(selected_file.file_name.clone()),
                        skip_cleanup: !viewer.cleanup_temp_files, // Invert: cleanup=true means skip=false
                    };

                    sender.oneshot_command(async move {
                        let res = executable_runner_service
                            .run_executable(executable_runner_model, None)
                            .await;

                        DocumentViewerCommandMsg::FinishedRunningViewer(res)
                    });
                }
            }
            DocumentViewerMsg::FileSelected => {
                self.selected_file = self.get_selected_file_info();
            }
            DocumentViewerMsg::ViewerSelected => {
                self.selected_viewer = self.get_selected_viewer();
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
                sender.input(DocumentViewerMsg::FetchViewers);
            }
            DocumentViewerMsg::FetchViewers => {
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
            DocumentViewerMsg::Ignore => {}
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
                tracing::info!("Viewers fetched successfully");
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
                tracing::error!(
                    error = ?error,
                    "Error fetching document viewers"
                );
                show_error_dialog(
                    format!(
                        "An error occurred while fetching document viewers: {}",
                        error
                    ),
                    root,
                );
            }
            DocumentViewerCommandMsg::FinishedRunningViewer(Ok(())) => {
                tracing::info!("Viewer executed successfully");
                root.close();
            }
            DocumentViewerCommandMsg::FinishedRunningViewer(Err(error)) => {
                show_error_dialog(
                    format!("An error occurred while running the viewer: {}", error),
                    root,
                );
            }
            DocumentViewerCommandMsg::Deleted(Ok(_)) => {
                tracing::info!("Viewer deleted successfully");
                // TODO: delete directly from the list?
                sender.input(DocumentViewerMsg::FetchViewers);
            }
            DocumentViewerCommandMsg::Deleted(Err(error)) => {
                show_error_dialog(
                    format!("An error occurred while deleting the viewer: {}", error),
                    root,
                );
            }
        }
    }
}

impl DocumentViewer {
    fn get_selected_file_info(&self) -> Option<FileSetFileInfoViewModel> {
        let selected_index = self.file_list_view_wrapper.selection_model.selected();
        let file_list_item = self.file_list_view_wrapper.get_visible(selected_index);
        if let (Some(item), Some(file_set)) = (file_list_item, &self.file_set) {
            let id = item.borrow().id;
            let file_info = file_set.files.iter().find(|f| f.file_info_id == id);
            file_info.cloned()
        } else {
            None
        }
    }

    fn get_selected_viewer(&self) -> Option<DocumentViewerViewModel> {
        let selected_index = self.viewer_list_view_wrapper.selection_model.selected();
        let viewer_list_item = self.viewer_list_view_wrapper.get_visible(selected_index);
        if let Some(item) = viewer_list_item {
            let id = item.borrow().id;
            let viewer = self.viewers.iter().find(|e| e.id == id);
            viewer.cloned()
        } else {
            None
        }
    }
}
