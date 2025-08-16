use std::sync::Arc;

use core_types::{DOCUMENT_FILE_TYPES, EMULATOR_FILE_TYPES, IMAGE_FILE_TYPES};
use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error,
    view_model_service::ViewModelService,
    view_models::{
        FileSetViewModel, ReleaseListModel, ReleaseViewModel, Settings, SoftwareTitleListModel,
    },
};

use crate::{
    document_file_set_viewer::{DocumentViewer, DocumentViewerInit},
    emulator_runner::{EmulatorRunnerInit, EmulatorRunnerModel},
    image_fileset_viewer::{ImageFileSetViewerInit, ImageFilesetViewer},
    image_viewer::{ImageViewer, ImageViewerInit, ImageViewerMsg},
    list_item::ListItem,
    release_form::{ReleaseFormInit, ReleaseFormModel, ReleaseFormOutputMsg},
    tabbed_image_viewer::{TabbedImageViewer, TabbedImageViewerInit, TabbedImageViewerMsg},
};

#[derive(Debug)]
pub struct ReleaseModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,

    selected_release: Option<ReleaseViewModel>,
    selected_release_system_names: String,

    emulator_file_set_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    image_file_set_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    document_file_set_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,

    selected_file_set: Option<FileSetViewModel>,
    selected_image_file_set: Option<FileSetViewModel>,
    selected_document_file_set: Option<FileSetViewModel>,
    emulator_runner: Option<Controller<EmulatorRunnerModel>>,
    image_file_set_viewer: Option<Controller<ImageFilesetViewer>>,
    document_file_set_viewer: Option<Controller<DocumentViewer>>,
    form_window: Option<Controller<ReleaseFormModel>>,
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
    StartImageFileSetViewer,
    StartDocumentFileSetViewer,
    StartEditRelease,
    UpdateRelease(ReleaseListModel),
    Clear,
    FileSetSelected {
        index: u32,
    },
    ImageFileSetSelected {
        index: u32,
    },
    DocumentFileSetSelected {
        index: u32,
    },
    ReleaseCreatedOrUpdated {
        id: i64,
    },
    SoftwareTitleCreated {
        software_title_list_model: SoftwareTitleListModel,
    },
}

#[derive(Debug)]
pub enum ReleaseCommandMsg {
    FetchedRelease(Result<ReleaseViewModel, Error>),
}

#[derive(Debug)]
pub enum ReleaseOutputMsg {
    SoftwareTitleCreated(SoftwareTitleListModel),
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
                        set_label: "Image File Sets:",
                    },
                    #[local_ref]
                    image_file_set_list_view -> gtk::ListView {
                        set_vexpand: true,
                    },

                    gtk::Button {
                        set_label: "View Image File Set",
                        #[watch]
                        set_sensitive: model.selected_image_file_set.is_some(),
                        connect_clicked => ReleaseMsg::StartImageFileSetViewer,
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

            gtk::Button {
                set_label: "Edit release",
                connect_clicked => ReleaseMsg::StartEditRelease,
            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let image_viewer_init = ImageViewerInit {
            settings: Arc::clone(&init_model.settings),
        };
        let tabbed_image_viewer_init = TabbedImageViewerInit {
            settings: Arc::clone(&init_model.settings),
        };
        let tabbed_image_viewer = TabbedImageViewer::builder()
            .launch(tabbed_image_viewer_init)
            .detach();
        let model = ReleaseModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,

            selected_release: None,
            selected_release_system_names: String::new(),
            emulator_file_set_list_view_wrapper: TypedListView::new(),
            image_file_set_list_view_wrapper: TypedListView::new(),
            document_file_set_list_view_wrapper: TypedListView::new(),

            selected_file_set: None,
            selected_image_file_set: None,
            selected_document_file_set: None,
            emulator_runner: None,
            image_file_set_viewer: None,
            tabbed_image_viewer,
            document_file_set_viewer: None,
            form_window: None,
        };

        let file_set_list_view = &model.emulator_file_set_list_view_wrapper.view;
        let selection_model = &model.emulator_file_set_list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |s| {
                let index = s.selected();
                println!("Selected index: {}", index);
                sender.input(ReleaseMsg::FileSetSelected { index });
            }
        ));

        let image_file_set_list_view = &model.image_file_set_list_view_wrapper.view;
        let image_selection_model = &model.image_file_set_list_view_wrapper.selection_model;
        image_selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |s| {
                let index = s.selected();
                println!("Selected index: {}", index);
                sender.input(ReleaseMsg::ImageFileSetSelected { index });
            }
        ));

        let document_file_set_list_view = &model.document_file_set_list_view_wrapper.view;
        let document_selection_model = &model.document_file_set_list_view_wrapper.selection_model;
        document_selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |s| {
                let index = s.selected();
                println!("Selected index: {}", index);
                sender.input(ReleaseMsg::DocumentFileSetSelected { index });
            }
        ));

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            ReleaseMsg::ReleaseSelected { id } => {
                sender.input(ReleaseMsg::FetchRelease { id });
            }
            ReleaseMsg::FetchRelease { id } => {
                let view_model_service = Arc::clone(&self.view_model_service);

                sender.oneshot_command(async move {
                    let release = view_model_service.get_release_view_model(id).await;
                    println!("Fetched release: {:?}", release);
                    ReleaseCommandMsg::FetchedRelease(release)
                });
            }
            ReleaseMsg::StartEmulatorRunner => {
                if let (Some(file_set), Some(release)) =
                    (&self.selected_file_set, &self.selected_release)
                {
                    println!("Starting emulator runner with file set: {:?}", file_set);
                    let init_model = EmulatorRunnerInit {
                        view_model_service: Arc::clone(&self.view_model_service),
                        repository_manager: Arc::clone(&self.repository_manager),
                        settings: Arc::clone(&self.settings),
                        file_set: file_set.clone(),
                        systems: release.systems.clone(),
                    };
                    let emulator_runner = EmulatorRunnerModel::builder()
                        .transient_for(root)
                        .launch(init_model)
                        .detach();

                    self.emulator_runner = Some(emulator_runner);
                    self.emulator_runner
                        .as_ref()
                        .expect("Emulator runner should be set already")
                        .widget()
                        .present();
                }
            }
            ReleaseMsg::StartImageFileSetViewer => {
                if let Some(file_set) = &self.selected_image_file_set {
                    println!(
                        "Starting image file set viewer with file set: {:?}",
                        file_set
                    );
                    let init_model = ImageFileSetViewerInit {
                        file_set: file_set.clone(),
                        settings: Arc::clone(&self.settings),
                    };
                    let image_file_set_viewer = ImageFilesetViewer::builder()
                        .transient_for(root)
                        .launch(init_model)
                        .detach();

                    self.image_file_set_viewer = Some(image_file_set_viewer);
                    self.image_file_set_viewer
                        .as_ref()
                        .expect("Image file set viewer should be set already")
                        .widget()
                        .present();
                }
            }
            ReleaseMsg::StartDocumentFileSetViewer => {
                if let Some(file_set) = &self.selected_document_file_set {
                    println!(
                        "Starting document file set viewer with file set: {:?}",
                        file_set
                    );
                    let init_model = DocumentViewerInit {
                        view_model_service: Arc::clone(&self.view_model_service),
                        repository_manager: Arc::clone(&self.repository_manager),
                        settings: Arc::clone(&self.settings),
                        file_set: file_set.clone(),
                    };
                    let document_file_set_viewer = DocumentViewer::builder()
                        .transient_for(root)
                        .launch(init_model)
                        .detach();

                    self.document_file_set_viewer = Some(document_file_set_viewer);
                    self.document_file_set_viewer
                        .as_ref()
                        .expect("Document file set viewer should be set already")
                        .widget()
                        .present();
                }
            }
            ReleaseMsg::UpdateRelease(release_list_model) => {
                println!("Updating release with model: {:?}", release_list_model);
                // TODO
            }
            ReleaseMsg::StartEditRelease => {
                if let Some(release) = &self.selected_release {
                    println!("Starting edit release for: {:?}", release);
                    let release_form_init_model = ReleaseFormInit {
                        view_model_service: Arc::clone(&self.view_model_service),
                        repository_manager: Arc::clone(&self.repository_manager),
                        settings: Arc::clone(&self.settings),
                        release: Some(release.clone()),
                    };
                    let form_window = ReleaseFormModel::builder()
                        .transient_for(root)
                        .launch(release_form_init_model)
                        .forward(sender.input_sender(), |msg| match msg {
                            ReleaseFormOutputMsg::ReleaseCreatedOrUpdated { id } => {
                                ReleaseMsg::FetchRelease { id }
                            }
                            ReleaseFormOutputMsg::SoftwareTitleCreated(
                                software_title_list_model,
                            ) => {
                                println!("Software title created: {:?}", software_title_list_model);
                                ReleaseMsg::SoftwareTitleCreated {
                                    software_title_list_model,
                                }
                            }
                        });

                    self.form_window = Some(form_window);

                    self.form_window
                        .as_ref()
                        .expect("Form window should be set already")
                        .widget()
                        .present();
                }
            }
            ReleaseMsg::Clear => {
                println!("Clearing release model");
                self.selected_release = None;
                self.selected_release_system_names.clear();
                self.emulator_file_set_list_view_wrapper.clear();
                self.image_file_set_list_view_wrapper.clear();
                self.document_file_set_list_view_wrapper.clear();
                self.selected_file_set = None;
                self.emulator_runner = None;
                self.form_window = None;
                self.tabbed_image_viewer.emit(TabbedImageViewerMsg::Clear);
            }
            ReleaseMsg::FileSetSelected { index } => {
                println!("File set selected with index: {}", index);
                let selected = self.emulator_file_set_list_view_wrapper.get(index);
                if let Some(file_set_list_item) = selected {
                    let file_set_id = file_set_list_item.borrow().id;
                    let file_set = self.selected_release.as_ref().and_then(|release| {
                        release
                            .file_sets
                            .iter()
                            .find(|fs| fs.id == file_set_id)
                            .cloned()
                    });
                    self.selected_file_set = file_set;
                    println!("Selected file set: {:?}", self.selected_file_set);
                } else {
                    println!("No file set found at index: {}", index);
                }
            }
            ReleaseMsg::ImageFileSetSelected { index } => {
                println!("Image file set selected with index: {}", index);
                let selected = self.image_file_set_list_view_wrapper.get(index);
                if let Some(file_set_list_item) = selected {
                    let file_set_id = file_set_list_item.borrow().id;
                    let file_set = self.selected_release.as_ref().and_then(|release| {
                        release
                            .file_sets
                            .iter()
                            .find(|fs| fs.id == file_set_id)
                            .cloned()
                    });

                    self.selected_image_file_set = file_set;
                    println!("Selected image file set: {:?}", self.selected_file_set);
                } else {
                    println!("No image file set found at index: {}", index);
                }
            }
            ReleaseMsg::DocumentFileSetSelected { index } => {
                println!("Document file set selected with index: {}", index);
                let selected = self.document_file_set_list_view_wrapper.get(index);
                if let Some(file_set_list_item) = selected {
                    let file_set_id = file_set_list_item.borrow().id;
                    let file_set = self.selected_release.as_ref().and_then(|release| {
                        release
                            .file_sets
                            .iter()
                            .find(|fs| fs.id == file_set_id)
                            .cloned()
                    });

                    self.selected_document_file_set = file_set;
                    println!("Selected document file set: {:?}", self.selected_file_set);
                } else {
                    println!("No document file set found at index: {}", index);
                }
            }
            ReleaseMsg::ReleaseCreatedOrUpdated { id } => {
                println!("Release created or updated with ID: {}", id);
                sender.input(ReleaseMsg::FetchRelease { id });
            }
            ReleaseMsg::SoftwareTitleCreated {
                software_title_list_model,
            } => {
                let res = sender.output(ReleaseOutputMsg::SoftwareTitleCreated(
                    software_title_list_model,
                ));
                if let Err(err) = res {
                    eprintln!("Error sending SoftwareTitleCreated output: {:?}", err);
                }
            }

            _ => (),
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            ReleaseCommandMsg::FetchedRelease(Ok(release)) => {
                println!("Release fetched successfully: {:?}", release);
                self.selected_release_system_names = release
                    .systems
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");

                let image_file_sets = release
                    .file_sets
                    .iter()
                    .filter(|fs| IMAGE_FILE_TYPES.contains(&fs.file_type.into()))
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
                    .filter(|fs| EMULATOR_FILE_TYPES.contains(&fs.file_type.into()))
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
                    .filter(|fs| IMAGE_FILE_TYPES.contains(&fs.file_type.into()))
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
                    .filter(|fs| DOCUMENT_FILE_TYPES.contains(&fs.file_type.into()))
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

                // Update the selected release

                self.selected_release = Some(release);
            }
            ReleaseCommandMsg::FetchedRelease(Err(err)) => {
                eprintln!("Error fetching release: {:?}", err);
                // TODO: show error to user
            }
        }
    }
}
