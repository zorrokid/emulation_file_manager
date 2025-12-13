use std::sync::Arc;

use core_types::{DOCUMENT_FILE_TYPES, EMULATOR_FILE_TYPES, IMAGE_FILE_TYPES};
use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error,
    file_set_download::service::DownloadService,
    view_model_service::ViewModelService,
    view_models::{
        FileSetViewModel, ReleaseListModel, ReleaseViewModel, Settings, SoftwareTitleListModel,
    },
};

use crate::{
    document_file_set_viewer::{DocumentViewer, DocumentViewerInit, DocumentViewerMsg},
    emulator_runner::{EmulatorRunnerInit, EmulatorRunnerModel, EmulatorRunnerMsg},
    image_fileset_viewer::{ImageFileSetViewerInit, ImageFilesetViewer, ImageFilesetViewerMsg},
    list_item::ListItem,
    tabbed_image_viewer::{
        TabbedImageViewer, TabbedImageViewerInit, TabbedImageViewerMsg, TabbedImageViewerOutputMsg,
    },
};

#[derive(Debug)]
pub struct ReleaseModel {
    view_model_service: Arc<ViewModelService>,

    selected_release: Option<ReleaseViewModel>,
    selected_release_system_names: String,

    emulator_file_set_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    image_file_set_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    document_file_set_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,

    selected_file_set: Option<FileSetViewModel>,
    selected_image_file_set: Option<FileSetViewModel>,
    selected_document_file_set: Option<FileSetViewModel>,
    emulator_runner: Controller<EmulatorRunnerModel>,
    image_file_set_viewer: Controller<ImageFilesetViewer>,
    document_file_set_viewer: Controller<DocumentViewer>,
    tabbed_image_viewer: Controller<TabbedImageViewer>,
}

#[derive(Debug)]
pub struct ReleaseInitModel {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub enum ReleaseMsg {
    ReleaseSelected {
        id: i64,
    },
    FetchRelease {
        id: i64,
    },
    StartEmulatorRunner,
    StartDocumentFileSetViewer,
    UpdateRelease(ReleaseListModel),
    Clear,
    FileSetSelected,
    ImageFileSetSelected,
    DocumentFileSetSelected,
    ReleaseCreatedOrUpdated {
        id: i64,
    },
    SoftwareTitleCreated {
        software_title_list_model: SoftwareTitleListModel,
    },
    SoftwareTitleUpdated {
        software_title_list_model: SoftwareTitleListModel,
    },
    ShowError(String),
}

#[derive(Debug)]
pub enum ReleaseCommandMsg {
    FetchedRelease(Result<ReleaseViewModel, Error>),
}

#[derive(Debug)]
pub enum ReleaseOutputMsg {
    SoftwareTitleCreated(SoftwareTitleListModel),
    ShowError(String),
}

#[relm4::component(pub)]
impl Component for ReleaseModel {
    type Input = ReleaseMsg;
    type Output = ReleaseOutputMsg;
    type CommandOutput = ReleaseCommandMsg;
    type Init = ReleaseInitModel;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            append = model.tabbed_image_viewer.widget(),
            set_spacing: 5,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 5,
                gtk::Label {
                    set_label: "Systems:",
                },
                gtk::Label {
                    #[watch]
                    set_label: model.selected_release_system_names.as_str(),
                },
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 5,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_spacing: 5,

                    gtk::Label {
                        set_label: "Emulator File Sets:",
                    },
                    #[local_ref]
                    file_set_list_view -> gtk::ListView {
                        set_vexpand: true,
                    },

                    gtk::Button {
                        set_label: "Run with Emulator",
                        #[watch]
                        set_sensitive: model.selected_file_set.is_some(),
                        connect_clicked => ReleaseMsg::StartEmulatorRunner,
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,

                    set_spacing: 5,

                    gtk::Label {
                        set_label: "Document File Sets:",
                    },
                    #[local_ref]
                    document_file_set_list_view -> gtk::ListView {
                        set_vexpand: true,
                    },

                    gtk::Button {
                        set_label: "View Document Set",
                        #[watch]
                        set_sensitive: model.selected_document_file_set.is_some(),
                        connect_clicked => ReleaseMsg::StartDocumentFileSetViewer,
                    },
                }
            },
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let download_service = Arc::new(DownloadService::new(
            Arc::clone(&init_model.repository_manager),
            Arc::clone(&init_model.settings),
        ));
        let tabbed_image_viewer_init = TabbedImageViewerInit {
            settings: Arc::clone(&init_model.settings),
            download_service: Arc::clone(&download_service),
        };
        let tabbed_image_viewer = TabbedImageViewer::builder()
            .launch(tabbed_image_viewer_init)
            .forward(sender.input_sender(), |msg| match msg {
                TabbedImageViewerOutputMsg::ShowError(err_msg) => ReleaseMsg::ShowError(err_msg),
            });

        let emulator_runner_init_model = EmulatorRunnerInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
            repository_manager: Arc::clone(&init_model.repository_manager),
            settings: Arc::clone(&init_model.settings),
        };
        let emulator_runner = EmulatorRunnerModel::builder()
            .transient_for(&root)
            .launch(emulator_runner_init_model)
            .detach();

        let document_viewer_init_model = DocumentViewerInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
            repository_manager: Arc::clone(&init_model.repository_manager),
            settings: Arc::clone(&init_model.settings),
        };
        let document_file_set_viewer = DocumentViewer::builder()
            .transient_for(&root)
            .launch(document_viewer_init_model)
            .detach();

        let image_file_set_viewer_init_model = ImageFileSetViewerInit {
            download_service: Arc::clone(&download_service),
        };
        let image_file_set_viewer = ImageFilesetViewer::builder()
            .transient_for(&root)
            .launch(image_file_set_viewer_init_model)
            .detach();

        let model = ReleaseModel {
            view_model_service: init_model.view_model_service,

            selected_release: None,
            selected_release_system_names: String::new(),
            emulator_file_set_list_view_wrapper: TypedListView::new(),
            image_file_set_list_view_wrapper: TypedListView::new(),
            document_file_set_list_view_wrapper: TypedListView::new(),

            selected_file_set: None,
            selected_image_file_set: None,
            selected_document_file_set: None,
            emulator_runner,
            image_file_set_viewer,
            tabbed_image_viewer,
            document_file_set_viewer,
        };

        let file_set_list_view = &model.emulator_file_set_list_view_wrapper.view;
        let selection_model = &model.emulator_file_set_list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ReleaseMsg::FileSetSelected);
            }
        ));

        let image_selection_model = &model.image_file_set_list_view_wrapper.selection_model;
        image_selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ReleaseMsg::ImageFileSetSelected);
            }
        ));

        let document_file_set_list_view = &model.document_file_set_list_view_wrapper.view;
        let document_selection_model = &model.document_file_set_list_view_wrapper.selection_model;
        document_selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ReleaseMsg::DocumentFileSetSelected);
            }
        ));

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ReleaseMsg::ReleaseSelected { id } => {
                sender.input(ReleaseMsg::FetchRelease { id });
            }
            ReleaseMsg::FetchRelease { id } => {
                let view_model_service = Arc::clone(&self.view_model_service);

                sender.oneshot_command(async move {
                    let release = view_model_service.get_release_view_model(id).await;
                    ReleaseCommandMsg::FetchedRelease(release)
                });
            }
            ReleaseMsg::StartEmulatorRunner => {
                if let (Some(file_set), Some(release)) =
                    (&self.selected_file_set, &self.selected_release)
                {
                    self.emulator_runner.emit(EmulatorRunnerMsg::Show {
                        file_set: file_set.clone(),
                        systems: release.systems.clone(),
                    });
                }
            }
            ReleaseMsg::StartDocumentFileSetViewer => {
                if let Some(file_set) = &self.selected_document_file_set {
                    tracing::info!("Starting document viewer for file set: {:?}", file_set.id);
                    self.document_file_set_viewer.emit(DocumentViewerMsg::Show {
                        file_set: file_set.clone(),
                    });
                }
            }
            ReleaseMsg::UpdateRelease(_) => {
                // TODO
            }
            ReleaseMsg::Clear => {
                self.selected_release = None;
                self.selected_release_system_names.clear();
                self.emulator_file_set_list_view_wrapper.clear();
                self.image_file_set_list_view_wrapper.clear();
                self.document_file_set_list_view_wrapper.clear();
                self.selected_file_set = None;
                self.tabbed_image_viewer.emit(TabbedImageViewerMsg::Clear);
            }
            ReleaseMsg::FileSetSelected => {
                self.selected_file_set = self.get_selected_file_set();
            }
            ReleaseMsg::ImageFileSetSelected => {
                self.selected_image_file_set = self.get_selected_image_file_set();
            }
            ReleaseMsg::DocumentFileSetSelected => {
                self.selected_document_file_set = self.get_selected_document_file_set();
            }
            ReleaseMsg::ReleaseCreatedOrUpdated { id } => {
                tracing::info!("Release created or updated with ID: {}", id);
                sender.input(ReleaseMsg::FetchRelease { id });
            }
            ReleaseMsg::SoftwareTitleCreated {
                software_title_list_model,
            } => {
                sender
                    .output(ReleaseOutputMsg::SoftwareTitleCreated(
                        software_title_list_model,
                    ))
                    .unwrap_or_else(|res| {
                        tracing::error!("Error sending SoftwareTitleCreated output: {:?}", res);
                    });
            }
            ReleaseMsg::ShowError(err_msg) => {
                sender
                    .output(ReleaseOutputMsg::ShowError(err_msg))
                    .unwrap_or_else(|res| {
                        tracing::error!("Error sending ShowError output: {:?}", res);
                    });
            }
            ReleaseMsg::SoftwareTitleUpdated {
                software_title_list_model: _,
            } => {
                // TODO
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
            ReleaseCommandMsg::FetchedRelease(Ok(release)) => {
                self.process_release(release);
            }
            ReleaseCommandMsg::FetchedRelease(Err(err)) => {
                let message = format!("Error fetching release: {:?}", err);
                tracing::error!(message);
                sender
                    .output(ReleaseOutputMsg::ShowError(message))
                    .unwrap_or_else(|e| {
                        tracing::error!("Failed to send ShowError output message: {:?}", e);
                    });
            }
        }
    }
}

impl ReleaseModel {
    fn process_release(&mut self, release: ReleaseViewModel) {
        tracing::info!("Fetched release with id: {}", release.id);
        self.selected_release_system_names = release
            .systems
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>()
            .join(", ");

        let image_file_sets = release
            .file_sets
            .iter()
            .filter(|fs| IMAGE_FILE_TYPES.contains(&fs.file_type))
            .cloned()
            .collect::<Vec<_>>();

        self.tabbed_image_viewer
            .emit(TabbedImageViewerMsg::SetFileSets {
                file_sets: image_file_sets,
            });

        // emulator file sets

        let emulator_file_sets = release
            .file_sets
            .iter()
            .filter(|fs| EMULATOR_FILE_TYPES.contains(&fs.file_type))
            .cloned()
            .collect::<Vec<_>>();

        let emulator_file_set_list_items = emulator_file_sets.iter().map(|fs| ListItem {
            id: fs.id,
            name: fs.file_set_name.clone(),
        });

        self.emulator_file_set_list_view_wrapper.clear();
        self.emulator_file_set_list_view_wrapper
            .extend_from_iter(emulator_file_set_list_items);

        let selected_index = self
            .emulator_file_set_list_view_wrapper
            .selection_model
            .selected();

        let selected_file_set_list_item =
            self.emulator_file_set_list_view_wrapper.get(selected_index);
        if let Some(file_set_list_item) = selected_file_set_list_item {
            let file_set = emulator_file_sets
                .iter()
                .find(|fs| fs.id == file_set_list_item.borrow().id);
            self.selected_file_set = file_set.cloned();
        } else {
            self.selected_file_set = None;
        }

        // image file sets

        let image_file_sets = release
            .file_sets
            .iter()
            .filter(|fs| IMAGE_FILE_TYPES.contains(&fs.file_type))
            .cloned()
            .collect::<Vec<_>>();

        let image_file_set_list_items = image_file_sets.iter().map(|fs| ListItem {
            id: fs.id,
            name: fs.file_set_name.clone(),
        });

        self.image_file_set_list_view_wrapper.clear();
        self.image_file_set_list_view_wrapper
            .extend_from_iter(image_file_set_list_items);

        let selected_index = self
            .image_file_set_list_view_wrapper
            .selection_model
            .selected();

        let selected_image_file_set_list_item =
            self.image_file_set_list_view_wrapper.get(selected_index);
        if let Some(file_set_list_item) = selected_image_file_set_list_item {
            let file_set = image_file_sets
                .iter()
                .find(|fs| fs.id == file_set_list_item.borrow().id);
            self.selected_image_file_set = file_set.cloned();
        } else {
            self.selected_image_file_set = None;
        }

        // document file sets

        let document_file_sets = release
            .file_sets
            .iter()
            .filter(|fs| DOCUMENT_FILE_TYPES.contains(&fs.file_type))
            .cloned()
            .collect::<Vec<_>>();

        let document_file_set_list_items = document_file_sets.iter().map(|fs| ListItem {
            id: fs.id,
            name: fs.file_set_name.clone(),
        });

        self.document_file_set_list_view_wrapper.clear();
        self.document_file_set_list_view_wrapper
            .extend_from_iter(document_file_set_list_items);

        let selected_index = self
            .document_file_set_list_view_wrapper
            .selection_model
            .selected();

        let selected_document_file_set_list_item =
            self.document_file_set_list_view_wrapper.get(selected_index);
        if let Some(file_set_list_item) = selected_document_file_set_list_item {
            let file_set = document_file_sets
                .iter()
                .find(|fs| fs.id == file_set_list_item.borrow().id);
            self.selected_document_file_set = file_set.cloned();
        } else {
            self.selected_document_file_set = None;
        }

        self.selected_release = Some(release);
    }

    fn get_selected_file_set(&self) -> Option<FileSetViewModel> {
        let selection = &self.emulator_file_set_list_view_wrapper.selection_model;
        if let Some(file_set_list_item) = self
            .emulator_file_set_list_view_wrapper
            .get_visible(selection.selected())
        {
            self.get_file_set_by_id(file_set_list_item.borrow().id)
        } else {
            None
        }
    }

    fn get_selected_image_file_set(&self) -> Option<FileSetViewModel> {
        let selection = &self.image_file_set_list_view_wrapper.selection_model;
        if let Some(file_set_list_item) = self
            .image_file_set_list_view_wrapper
            .get_visible(selection.selected())
        {
            self.get_file_set_by_id(file_set_list_item.borrow().id)
        } else {
            None
        }
    }

    fn get_selected_document_file_set(&self) -> Option<FileSetViewModel> {
        let selection = &self.document_file_set_list_view_wrapper.selection_model;
        if let Some(file_set_list_item) = self
            .document_file_set_list_view_wrapper
            .get_visible(selection.selected())
        {
            self.get_file_set_by_id(file_set_list_item.borrow().id)
        } else {
            None
        }
    }

    fn get_file_set_by_id(&self, file_set_id: i64) -> Option<FileSetViewModel> {
        self.selected_release.as_ref().and_then(|release| {
            release
                .file_sets
                .iter()
                .find(|fs| fs.id == file_set_id)
                .cloned()
        })
    }
}
