mod document_file_set_viewer;
mod document_viewer_form;
mod emulator_form;
mod emulator_runner;
mod file_importer;
mod file_selector;
mod file_set_form;
mod image_fileset_viewer;
mod image_viewer;
mod list_item;
mod release;
mod release_form;
mod releases;
mod software_title_selector;
mod system_selector;
mod tabbed_image_viewer;
mod utils;
use std::sync::Arc;

use database::{get_db_pool, repository_manager::RepositoryManager};
use list_item::ListItem;
use release::{ReleaseInitModel, ReleaseModel, ReleaseMsg, ReleaseOutputMsg};
use releases::{ReleasesInit, ReleasesModel, ReleasesMsg, ReleasesOutputMsg};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    gtk::{self, glib::clone, prelude::*},
    once_cell::sync::OnceCell,
    typed_view::list::TypedListView,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{Settings, SoftwareTitleListModel},
};

#[derive(Debug)]
struct InitResult {
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    software_titles: Vec<SoftwareTitleListModel>,
    settings: Arc<Settings>,
}

#[derive(Debug)]
enum AppMsg {
    Initialize,
    SoftwareTitleSelected { index: u32 },
    SoftwareTitleCreated(SoftwareTitleListModel),
    ReleaseSelected { id: i64 },
}

#[derive(Debug)]
enum CommandMsg {
    InitializationDone(InitResult),
}

struct AppModel {
    repository_manager: OnceCell<Arc<RepositoryManager>>,
    view_model_service: OnceCell<Arc<ViewModelService>>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    releases_view: gtk::Box,
    releases: OnceCell<Controller<ReleasesModel>>,
    release_view: gtk::Box,
    release: OnceCell<Controller<ReleaseModel>>,
}

struct AppWidgets {}

impl Component for AppModel {
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = CommandMsg;
    type Init = ();
    type Root = gtk::Window;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("EFCM")
            .default_width(800)
            .default_height(800)
            .build()
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::with_sorting();

        let main_layout_hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .build();

        let left_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .build();

        left_vbox.set_width_request(300);

        let right_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        main_layout_hbox.append(&left_vbox);
        main_layout_hbox.append(&right_vbox);

        let title_label = gtk::Label::builder().label("Software Titles").build();
        left_vbox.append(&title_label);

        let software_titles_list_container = gtk::ScrolledWindow::builder().vexpand(true).build();

        let software_titles_view = &list_view_wrapper.view;

        let selection_model = &list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(AppMsg::SoftwareTitleSelected {
                    index: selection.selected(),
                });
            }
        ));

        software_titles_list_container.set_child(Some(software_titles_view));

        left_vbox.append(&software_titles_list_container);

        root.set_child(Some(&main_layout_hbox));

        let widgets = AppWidgets {};

        let model = AppModel {
            repository_manager: OnceCell::new(),
            view_model_service: OnceCell::new(),
            list_view_wrapper,
            releases_view: left_vbox,
            release_view: right_vbox,
            releases: OnceCell::new(),
            release: OnceCell::new(),
        };

        sender.input(AppMsg::Initialize);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            AppMsg::Initialize => {
                sender.oneshot_command(async {
                    let pool = get_db_pool().await.expect("DB pool initialization failed");
                    let repository_manager = Arc::new(RepositoryManager::new(pool));
                    let view_model_service =
                        Arc::new(ViewModelService::new(Arc::clone(&repository_manager)));
                    let software_titles = view_model_service
                        .get_software_title_list_models()
                        .await
                        .expect("Fetching software titles failed");
                    let settings = view_model_service
                        .get_settings()
                        .await
                        .expect("Failed to get config");
                    let settings = Arc::new(settings);
                    CommandMsg::InitializationDone(InitResult {
                        repository_manager,
                        view_model_service,
                        software_titles,
                        settings,
                    })
                });
            }
            AppMsg::SoftwareTitleSelected { index } => {
                if let Some(title) = self.list_view_wrapper.get_visible(index) {
                    let title = title.borrow();
                    println!("Selected software title: {}", title.name);
                    self.releases
                        .get()
                        .expect("ReleasesModel not initialized")
                        .emit(ReleasesMsg::SoftwareTitleSelected { id: title.id });
                    self.release
                        .get()
                        .expect("Release widget not initialized")
                        .sender()
                        .emit(ReleaseMsg::Clear);
                } else {
                    println!("No software title found at index {}", index);
                }
            }
            AppMsg::SoftwareTitleCreated(software_title_list_model) => {
                self.list_view_wrapper.append(ListItem {
                    id: software_title_list_model.id,
                    name: software_title_list_model.name.clone(),
                });
            }
            AppMsg::ReleaseSelected { id } => {
                self.release
                    .get()
                    .expect("ReleasesModel not initialized")
                    .emit(ReleaseMsg::ReleaseSelected { id });
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
            CommandMsg::InitializationDone(init_result) => {
                let view_model_service = Arc::clone(&init_result.view_model_service);
                let repository_manager = Arc::clone(&init_result.repository_manager);
                let list_items = init_result.software_titles.iter().map(|title| ListItem {
                    name: title.name.clone(),
                    id: title.id,
                });
                self.list_view_wrapper.extend_from_iter(list_items);

                let releases_init = ReleasesInit {
                    view_model_service,
                    repository_manager,
                    settings: Arc::clone(&init_result.settings),
                };

                let releases = ReleasesModel::builder().launch(releases_init).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        ReleasesOutputMsg::SoftwareTitleCreated {
                            software_title_list_model,
                        } => AppMsg::SoftwareTitleCreated(software_title_list_model),
                        ReleasesOutputMsg::ReleaseSelected { id } => AppMsg::ReleaseSelected { id },
                    },
                );
                self.releases_view.append(releases.widget());

                let release_init_model = ReleaseInitModel {
                    view_model_service: Arc::clone(&init_result.view_model_service),
                    repository_manager: Arc::clone(&init_result.repository_manager),
                    settings: Arc::clone(&init_result.settings),
                };
                let release_model = ReleaseModel::builder().launch(release_init_model).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        ReleaseOutputMsg::SoftwareTitleCreated(software_title_list_model) => {
                            AppMsg::SoftwareTitleCreated(software_title_list_model)
                        }
                    },
                );
                self.release_view.append(release_model.widget());

                self.view_model_service
                    .set(init_result.view_model_service)
                    .expect("view model service already initialized?");
                self.repository_manager
                    .set(init_result.repository_manager)
                    .expect("repository manger already initialized");

                self.releases
                    .set(releases)
                    .expect("ReleasesModel already initialized");

                self.release
                    .set(release_model)
                    .expect("ReleaseModel already initialized");
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("org.zorrokid.efcm");
    app.run::<AppModel>(());
}
