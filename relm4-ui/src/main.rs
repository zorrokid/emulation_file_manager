mod file_importer;
mod file_selector;
mod file_set_form;
mod list_item;
mod release_form;
mod releases;
mod software_title_selector;
mod system_selector;
mod utils;
use std::sync::Arc;

use database::{get_db_pool, repository_manager::RepositoryManager};
use list_item::ListItem;
use releases::{ReleasesInit, ReleasesModel, ReleasesMsg};
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
    settings: Settings,
}

#[derive(Debug)]
enum AppMsg {
    Initialize,
    SoftwareTitleSelected { index: u32 },
    AddSoftwareTitle { name: String },
    Dummy,
}

#[derive(Debug)]
enum CommandMsg {
    InitializationDone(InitResult),
    SoftwareTitleAdded(ListItem),
}

struct AppModel {
    software_titles: Vec<SoftwareTitleListModel>,
    repository_manager: OnceCell<Arc<RepositoryManager>>,
    view_model_service: OnceCell<Arc<ViewModelService>>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    releases_view: gtk::Box,
    releases: OnceCell<Controller<ReleasesModel>>,
}

struct AppWidgets {
    //label: gtk::Label,
    //releases_view: gtk::Box,
}

impl Component for AppModel {
    /// The type of the messages that this component can receive.
    type Input = AppMsg;
    /// The type of the messages that this component can send.
    type Output = ();
    type CommandOutput = CommandMsg;
    /// The type of data with which this component will be initialized.
    type Init = u8;
    /// The root GTK widget that this component will create.
    type Root = gtk::Window;
    /// A data structure that contains the widgets that you will need to update.
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("EFCM")
            .default_width(800)
            .default_height(800)
            .build()
    }

    fn init(
        counter: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize the ListView wrapper
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> = TypedListView::new();

        let main_layout_hbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(10)
            .build();

        let left_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let right_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        main_layout_hbox.append(&left_vbox);
        main_layout_hbox.append(&right_vbox);

        let title_label = gtk::Label::builder().label("Software Titles").build();
        left_vbox.append(&title_label);

        let add_new_software_title_entry = gtk::Entry::builder()
            .placeholder_text("Add new software title")
            .build();

        add_new_software_title_entry.connect_activate(clone!(
            #[strong]
            sender,
            move |entry| {
                let buffer = entry.buffer();
                sender.input(AppMsg::AddSoftwareTitle {
                    name: buffer.text().into(),
                });
                buffer.delete_text(0, None);
            }
        ));

        left_vbox.append(&add_new_software_title_entry);

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
            software_titles: vec![],
            repository_manager: OnceCell::new(),
            view_model_service: OnceCell::new(),
            list_view_wrapper,
            releases_view: right_vbox,
            releases: OnceCell::new(),
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
                    CommandMsg::InitializationDone(InitResult {
                        repository_manager,
                        view_model_service,
                        software_titles,
                        settings,
                    })
                });
            }
            AppMsg::SoftwareTitleSelected { index } => {
                if let Some(title) = self.list_view_wrapper.get(index) {
                    let title = title.borrow();
                    println!("Selected software title: {}", title.name);
                    self.releases
                        .get()
                        .expect("ReleasesModel not initialized")
                        .emit(ReleasesMsg::SoftwareTitleSelected { id: title.id });
                } else {
                    println!("No software title found at index {}", index);
                }
            }
            AppMsg::AddSoftwareTitle { name } => {
                let repository_manager = self
                    .repository_manager
                    .get()
                    .expect("RepositoryManager not initialized");

                sender.oneshot_command(clone!(
                    #[strong]
                    repository_manager,
                    async move {
                        let id = repository_manager
                            .get_software_title_repository()
                            .add_software_title(&name, None)
                            .await
                            .expect("Failed to add software title");

                        CommandMsg::SoftwareTitleAdded(ListItem { id, name })
                    }
                ));
            }
            AppMsg::Dummy => {
                println!("Dummy message received");
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
                self.view_model_service
                    .set(init_result.view_model_service)
                    .expect("view model service already initialized?");
                self.repository_manager
                    .set(init_result.repository_manager)
                    .expect("repository manger already initialized");
                self.software_titles = init_result.software_titles;
                let list_items = self.software_titles.iter().map(|title| ListItem {
                    name: title.name.clone(),
                    id: title.id,
                });
                self.list_view_wrapper.extend_from_iter(list_items);

                let releases_init = ReleasesInit {
                    view_model_service,
                    repository_manager,
                    settings: Arc::new(init_result.settings),
                };

                let releases = ReleasesModel::builder().launch(releases_init).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        _ => AppMsg::Dummy, // Example message forwarding
                    },
                );
                self.releases_view.append(releases.widget());
                self.releases
                    .set(releases)
                    .expect("ReleasesModel already initialized");
            }
            CommandMsg::SoftwareTitleAdded(item) => {
                self.software_titles.push(SoftwareTitleListModel {
                    id: item.id,
                    name: item.name.clone(),
                    can_delete: false,
                });
                self.list_view_wrapper.append(item);
            }
        }
    }

    fn update_view(&self, _widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        // Update the label with the current counter value
        //widgets.label.set_label(&format!("Counter: {}", self.counter));
        // Update the releases view if needed
        //widgets.releases_view.set_child(Some(self.releases.widget()));
    }
}

fn main() {
    let app = RelmApp::new("org.zorrokid.efcm");
    app.run::<AppModel>(0);
}
