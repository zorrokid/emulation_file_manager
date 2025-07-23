use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, EntryBufferExtManual, EntryExt, GtkWindowExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError, view_model_service::ViewModelService,
    view_models::SystemListModel,
};

use crate::list_item::ListItem;

#[derive(Debug)]
pub enum SystemSelectMsg {
    FetchSystems,
    AddSystem { name: String },
    SelectClicked,
}

#[derive(Debug)]
pub enum SystemSelectOutputMsg {
    SystemSelected(SystemListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    SystemsFetched(Result<Vec<SystemListModel>, ServiceError>),
    SystemAdded(SystemListModel),
    AddingSystemFailed(DatabaseError),
}

pub struct SystemSelectInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub struct SystemSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    systems: Vec<SystemListModel>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

#[derive(Debug)]
pub struct Widgets {}

impl Component for SystemSelectModel {
    type Input = SystemSelectMsg;
    type Output = SystemSelectOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = SystemSelectInit;
    type Widgets = Widgets;
    type Root = gtk::Window;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("Release Form")
            .default_width(800)
            .default_height(800)
            .build()
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> = TypedListView::new();

        /*let selection_model = &list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |selection| {
                sender.input(SystemSelectMsg::SystemSelected {
                    index: selection.selected(),
                });
            }
        ));*/

        let system_list = &list_view_wrapper.view;
        let system_list_container = gtk::ScrolledWindow::builder().vexpand(true).build();
        system_list_container.set_child(Some(system_list));

        let v_box = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let label = gtk::Label::new(Some("System Select Component"));

        v_box.append(&label);

        let add_new_system_entry = gtk::Entry::builder()
            .placeholder_text("Add new system")
            .build();

        add_new_system_entry.connect_activate(clone!(
            #[strong]
            sender,
            move |entry| {
                let buffer = entry.buffer();
                sender.input(SystemSelectMsg::AddSystem {
                    name: buffer.text().into(),
                });
                buffer.delete_text(0, None);
            }
        ));

        v_box.append(&add_new_system_entry);
        v_box.append(&system_list_container);
        let select_button = gtk::Button::with_label("Select System");
        select_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(SystemSelectMsg::SelectClicked);
            }
        ));
        v_box.append(&select_button);

        root.set_child(Some(&v_box));

        let widgets = Widgets {};

        let model = SystemSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            systems: Vec::new(),
            list_view_wrapper,
        };
        sender.input(SystemSelectMsg::FetchSystems);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            SystemSelectMsg::FetchSystems => {
                println!("Fetching systems...");
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let systems_result = view_model_service.get_system_list_models().await;
                    CommandMsg::SystemsFetched(systems_result)
                });
            }
            SystemSelectMsg::AddSystem { name } => {
                println!("Adding new system: {}", name);
                let repository_manager = Arc::clone(&self.repository_manager);
                sender.oneshot_command(async move {
                    let result = repository_manager
                        .get_system_repository()
                        .add_system(&name)
                        .await;
                    match result {
                        Ok(id) => {
                            let system_list_model = SystemListModel {
                                id,
                                name: name.clone(),
                                can_delete: true, // OK to delete since this was just added
                            };
                            CommandMsg::SystemAdded(system_list_model)
                        }
                        Err(e) => CommandMsg::AddingSystemFailed(e),
                    }
                });
            }
            SystemSelectMsg::SelectClicked => {
                let selected = self.list_view_wrapper.selection_model.selected();
                if let Some(system) = self.list_view_wrapper.get(selected) {
                    let system = system.borrow();
                    println!("System selected: {} with ID: {}", system.name, system.id);
                    let res =
                        sender.output(SystemSelectOutputMsg::SystemSelected(SystemListModel {
                            id: system.id,
                            name: system.name.clone(),
                            can_delete: false, // TODO
                        }));
                    match res {
                        Ok(_) => {
                            println!("System selection output sent successfully.");
                            root.close();
                        }
                        Err(e) => eprintln!("Failed to send system selection output: {:?}", e),
                    }
                } else {
                    eprintln!("No system found at selected index {}", selected);
                }
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
            CommandMsg::SystemsFetched(Ok(systems)) => {
                // Handle the fetched systems, e.g., populate a dropdown or list
                for system in &systems {
                    println!("Fetched system: {} with ID: {}", system.name, system.id);
                }
                self.systems = systems;
                let list_items = self.systems.iter().map(|system| ListItem {
                    name: system.name.clone(),
                    id: system.id,
                });
                self.list_view_wrapper.extend_from_iter(list_items);
            }
            CommandMsg::SystemsFetched(Err(e)) => {
                // Handle the error
                eprintln!("Error fetching systems: {:?}", e);
            }
            CommandMsg::SystemAdded(system_list_model) => {
                // Handle the successful addition of a system
                println!("Successfully added system: {}", system_list_model.name);
                sender.input(SystemSelectMsg::FetchSystems);
            }
            CommandMsg::AddingSystemFailed(error) => {
                // Handle the error when adding a system
                eprintln!("Error adding system: {:?}", error);
            }
        }
    }
}
