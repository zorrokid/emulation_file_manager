use std::sync::Arc;

use database::{get_db_pool, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender, RelmApp, RelmWidgetExt,
    gtk::{self, glib::clone, prelude::*},
    once_cell::sync::OnceCell,
    typed_view::list::{RelmListItem, TypedListView},
};
use service::{view_model_service::ViewModelService, view_models::SoftwareTitleListModel};

#[derive(Debug)]
struct InitResult {
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    software_titles: Vec<SoftwareTitleListModel>,
}

#[derive(Debug)]
enum AppMsg {
    Increment,
    Decrement,
    Initialize,
    SoftwareTitleSelected { index: u32 },
    AddSoftwareTitle { name: String },
}

#[derive(Debug)]
enum CommandMsg {
    InitializationDone(InitResult),
    SoftwareTitleAdded(SoftwareListItem),
}

#[derive(Debug, PartialEq, Eq)]
struct SoftwareListItem {
    id: i64,
    title: String,
    description: String,
}

struct Widgets {
    label: gtk::Label,
}

impl RelmListItem for SoftwareListItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
            my_box = gtk::Box {
                #[name = "label"]
                gtk::Label,
            }
        }

        let widgets = Widgets { label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let Widgets { label } = widgets;
        label.set_label(&format!("Name: {} ", self.title));
    }
}

struct AppModel {
    counter: u8,
    software_titles: Vec<SoftwareTitleListModel>,
    repository_manager: OnceCell<Arc<RepositoryManager>>,
    view_model_service: OnceCell<Arc<ViewModelService>>,
    list_view_wrapper: TypedListView<SoftwareListItem, gtk::SingleSelection>,
}

struct AppWidgets {
    label: gtk::Label,
}

#[relm4::component]
impl Component for AppModel {
    /// The type of the messages that this component can receive.
    type Input = AppMsg;
    /// The type of the messages that this component can send.
    type Output = ();
    type CommandOutput = CommandMsg;
    /// The type of data with which this component will be initialized.
    type Init = u8;

    view! {
        gtk::Window {
            set_title: Some("EFCM"),
            set_default_width: 300,
            set_default_height: 100,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,

                gtk::Button {
                    set_label: "Increment",
                    connect_clicked => AppMsg::Increment
                },

                gtk::Button::with_label("Decrement") {
                    connect_clicked => AppMsg::Decrement
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!("Counter: {}", model.counter),
                    set_margin_all: 5,
                },
                gtk::Entry {
                    connect_activate[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(AppMsg::AddSoftwareTitle {name: buffer.text().into() });
                        buffer.delete_text(0, None);
                    }
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,

                    #[local_ref]
                    software_titles_view -> gtk::ListView {}
                }
            }
        }
    }

    fn init(
        counter: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize the ListView wrapper
        let list_view_wrapper: TypedListView<SoftwareListItem, gtk::SingleSelection> =
            TypedListView::new();

        let model = AppModel {
            counter,
            software_titles: vec![],
            repository_manager: OnceCell::new(),
            view_model_service: OnceCell::new(),
            list_view_wrapper,
        };

        // macro code generation
        let software_titles_view = &model.list_view_wrapper.view;

        let selection_model = &model.list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(AppMsg::SoftwareTitleSelected {
                    index: selection.selected(),
                });
            }
        ));
        let widgets = view_output!();

        sender.input(AppMsg::Initialize);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
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
                    CommandMsg::InitializationDone(InitResult {
                        repository_manager,
                        view_model_service,
                        software_titles,
                    })
                });
            }
            AppMsg::SoftwareTitleSelected { index } => {
                if let Some(title) = self.list_view_wrapper.get(index) {
                    let title = title.borrow();
                    println!("Selected software title: {}", title.title);
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

                        CommandMsg::SoftwareTitleAdded(SoftwareListItem {
                            id,
                            title: name,
                            description: "".to_string(),
                        })
                    }
                ));
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            CommandMsg::InitializationDone(init_result) => {
                self.view_model_service
                    .set(init_result.view_model_service)
                    .expect("view model service already initialized?");
                self.repository_manager
                    .set(init_result.repository_manager)
                    .expect("repository manger already initialized");
                self.software_titles = init_result.software_titles;
                let list_items = self.software_titles.iter().map(|title| SoftwareListItem {
                    title: title.name.clone(),
                    description: title.name.clone(),
                    id: title.id,
                });
                self.list_view_wrapper.extend_from_iter(list_items);
            }
            CommandMsg::SoftwareTitleAdded(item) => {
                self.software_titles.push(SoftwareTitleListModel {
                    id: item.id,
                    name: item.title.clone(),
                    can_delete: false,
                });
                self.list_view_wrapper.append(item);
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("org.zorrokid.efcm");
    app.run::<AppModel>(0);
}
