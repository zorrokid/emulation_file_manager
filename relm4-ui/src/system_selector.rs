use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        prelude::{
            ButtonExt, EntryBufferExtManual, EntryExt, GtkWindowExt, OrientableExt, WidgetExt,
        },
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
    pub selected_system_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct SystemSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    systems: Vec<SystemListModel>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_system_ids: Vec<i64>,
}

#[relm4::component(pub)]
impl Component for SystemSelectModel {
    type Input = SystemSelectMsg;
    type Output = SystemSelectOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = SystemSelectInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 800,
            set_title: Some("System Selector"),
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Label {
                    set_label: "System Selector",
                },

                gtk::Entry {
                    connect_activate[sender] => move |entry|  {
                        let buffer = entry.buffer();
                        sender.input(
                            SystemSelectMsg::AddSystem {
                                name: buffer.text().into(),
                            }
                        );
                        buffer.delete_text(0, None);
                    }
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    systems_list_view -> gtk::ListView {}
                },

                gtk::Button {
                    set_label: "Select System",
                    connect_clicked => SystemSelectMsg::SelectClicked,
                },

            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::with_sorting();

        let model = SystemSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            systems: Vec::new(),
            list_view_wrapper,
            selected_system_ids: init_model.selected_system_ids,
        };

        let systems_list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();

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
                if let Some(system) = self.list_view_wrapper.get_visible(selected) {
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
                let list_items = self
                    .systems
                    .iter()
                    .filter(|f| !self.selected_system_ids.contains(&f.id))
                    .map(|system| ListItem {
                        name: system.name.clone(),
                        id: system.id,
                    });
                self.list_view_wrapper.clear();
                self.list_view_wrapper.extend_from_iter(list_items);
            }
            CommandMsg::SystemsFetched(Err(e)) => {
                eprintln!("Error fetching systems: {:?}", e);
                // TODO: show error to user
            }
            CommandMsg::SystemAdded(system_list_model) => {
                println!("Successfully added system: {}", system_list_model.name);
                sender.input(SystemSelectMsg::FetchSystems);
            }
            CommandMsg::AddingSystemFailed(error) => {
                eprintln!("Error adding system: {:?}", error);
                // TODO: show error to user
            }
        }
    }
}
