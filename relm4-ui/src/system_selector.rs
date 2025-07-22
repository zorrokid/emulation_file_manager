use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self, gio,
        prelude::{BoxExt, GtkWindowExt},
    },
};
use service::{error::Error, view_model_service::ViewModelService, view_models::SystemListModel};

pub struct SystemListItem {
    name: String,
    id: i64,
}

#[derive(Debug)]
pub enum SystemSelectMsg {
    FetchSystems,
}

#[derive(Debug)]
pub enum SystemSelectOutputMsg {
    SystemSelected(SystemListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    SystemsFetched(Result<Vec<SystemListModel>, Error>),
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

        let label = gtk::Label::new(Some("Release Form Component"));

        v_box.append(&label);

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
        }
    }
}
