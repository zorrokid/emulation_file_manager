use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self, gio,
        glib::clone,
        prelude::{BoxExt, EntryBufferExtManual, EntryExt, GtkWindowExt},
    },
};
use service::{
    error::Error as ServiceError, view_model_service::ViewModelService,
    view_models::SystemListModel,
};

pub struct SystemListItem {
    name: String,
    id: i64,
}

#[derive(Debug)]
pub enum SystemSelectMsg {
    FetchSystems,
    AddSystem { name: String },
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
        println!("Initializing SystemSelectModel");
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

        /*let list_store = gio::ListStore::new::<SystemListItem>();

        let systems_dropdown = gtk::DropDown::builder()
            .model(&gtk::StringList::new(&["System 1", "System 2", "System 3"]))
            .build();

        v_box.append(&systems_dropdown);*/

        root.set_child(Some(&v_box));

        let widgets = Widgets {};

        let model = SystemSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            systems: Vec::new(),
        };
        sender.input(SystemSelectMsg::FetchSystems);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
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
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            CommandMsg::SystemsFetched(Ok(systems)) => {
                // Handle the fetched systems, e.g., populate a dropdown or list
                for system in &systems {
                    println!("Fetched system: {} with ID: {}", system.name, system.id);
                }
                self.systems = systems;
            }
            CommandMsg::SystemsFetched(Err(e)) => {
                // Handle the error
                eprintln!("Error fetching systems: {:?}", e);
            }
            CommandMsg::SystemAdded(system_list_model) => {
                // Handle the successful addition of a system
                println!("Successfully added system: {}", system_list_model.name);
                // Optionally, you could fetch the updated list of systems
                // self.update(SystemSelectMsg::FetchSystems, sender, root);
            }
            CommandMsg::AddingSystemFailed(error) => {
                // Handle the error when adding a system
                eprintln!("Error adding system: {:?}", error);
            }
        }
    }
}
