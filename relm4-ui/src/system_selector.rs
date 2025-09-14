use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self, glib,
        prelude::{
            BoxExt as _, ButtonExt, EntryBufferExtManual, EntryExt, GtkWindowExt, OrientableExt,
            WidgetExt,
        },
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error as ServiceError, view_model_service::ViewModelService,
    view_models::SystemListModel,
};
use ui_components::confirm_dialog::{
    ConfirmDialog, ConfirmDialogInit, ConfirmDialogMsg, ConfirmDialogOutputMsg,
};

use crate::{
    list_item::ListItem,
    system_form::{SystemFormInit, SystemFormModel, SystemFormMsg, SystemFormOutputMsg},
};

#[derive(Debug)]
pub enum SystemSelectMsg {
    FetchSystems,
    AddSystem { name: String },
    SelectClicked,
    EditClicked,
    DeleteClicked,
    DeleteConfirmed,
    Ignore,
    AddClicked,
    Show { selected_system_ids: Vec<i64> },
    Hide,
    SystemAdded(SystemListModel),
    SystemUpdated(SystemListModel),
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
    Deleted(Result<(), DatabaseError>),
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
    selected_system_ids: Vec<i64>,
    system_form_controller: Controller<SystemFormModel>,
    confirm_dialog_controller: Controller<ConfirmDialog>,
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
            set_default_width: 400,
            set_default_height: 600,
            set_title: Some("System Selector"),

            connect_close_request[sender] => move |_| {
                sender.input(SystemSelectMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

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

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,

                    gtk::Button {
                        set_label: "Add",
                        connect_clicked => SystemSelectMsg::AddClicked,
                    },
                    gtk::Button {
                        set_label: "Edit",
                        connect_clicked => SystemSelectMsg::EditClicked,
                    },
                    gtk::Button {
                        set_label: "Delete",
                        connect_clicked => SystemSelectMsg::DeleteClicked,
                    },
                    gtk::Button {
                        set_label: "Select",
                        connect_clicked => SystemSelectMsg::SelectClicked,
                    },
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

        // TODO: instantiate in update when needed?
        let confirm_dialog_controller = ConfirmDialog::builder()
            .transient_for(&root)
            .launch(ConfirmDialogInit {
                title: "Confirm Deletion".to_string(),
                visible: false,
            })
            .forward(sender.input_sender(), |msg| match msg {
                ConfirmDialogOutputMsg::Confirmed => SystemSelectMsg::DeleteConfirmed,
                ConfirmDialogOutputMsg::Canceled => SystemSelectMsg::Ignore,
            });

        let system_form_controller = SystemFormModel::builder()
            .transient_for(&root)
            .launch(SystemFormInit {
                repository_manager: Arc::clone(&init_model.repository_manager),
            })
            .forward(sender.input_sender(), |msg| match msg {
                SystemFormOutputMsg::SystemAdded(software_title_list_model) => {
                    SystemSelectMsg::SystemAdded(software_title_list_model)
                }
                SystemFormOutputMsg::SystemUpdated(software_title_list_model) => {
                    SystemSelectMsg::SystemUpdated(software_title_list_model)
                }
            });

        let model = SystemSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            systems: Vec::new(),
            list_view_wrapper,
            selected_system_ids: Vec::new(),
            system_form_controller,
            confirm_dialog_controller,
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
            SystemSelectMsg::Show {
                selected_system_ids,
            } => {
                self.selected_system_ids = selected_system_ids;
                sender.input(SystemSelectMsg::FetchSystems);
                root.show();
            }
            SystemSelectMsg::Hide => {
                root.hide();
            }
            SystemSelectMsg::AddClicked => {
                self.system_form_controller
                    .emit(SystemFormMsg::Show { edit_system: None });
            }
            SystemSelectMsg::EditClicked => {
                let edit_system = self
                    .list_view_wrapper
                    .get_visible(self.list_view_wrapper.selection_model.selected())
                    .and_then(|st| {
                        // TODO: probably local systems collection not needed, list view item
                        // should have all needed data
                        self.systems
                            .iter()
                            .find(|s| s.id == st.borrow().id)
                            .cloned()
                    });
                if let Some(edit_system) = &edit_system {
                    self.system_form_controller.emit(SystemFormMsg::Show {
                        edit_system: Some(edit_system.clone()),
                    });
                }
            }
            SystemSelectMsg::DeleteClicked => {
                self.confirm_dialog_controller.emit(ConfirmDialogMsg::Show);
            }
            SystemSelectMsg::DeleteConfirmed => {
                let repository_manager = Arc::clone(&self.repository_manager);
                let selected = self.list_view_wrapper.selection_model.selected();
                let system = self.list_view_wrapper.get_visible(selected);
                let id = system.as_ref().map(|st| st.borrow().id);
                if let Some(id) = id {
                    sender.oneshot_command(async move {
                        println!("Deleting system with ID {}", id);
                        let result = repository_manager
                            .get_system_repository()
                            .delete_system(id)
                            .await;
                        CommandMsg::Deleted(result)
                    });
                }
            }
            SystemSelectMsg::SystemAdded(_system_list_model) => {
                // TODO: add system to the list directly
                sender.input(SystemSelectMsg::FetchSystems);
            }
            SystemSelectMsg::SystemUpdated(_system_list_model) => {
                // TODO: update system in the list directly
                sender.input(SystemSelectMsg::FetchSystems);
            }
            SystemSelectMsg::Ignore => (),
            _ => (),
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
            CommandMsg::Deleted(Ok(_id)) => {
                println!("Successfully deleted system");
                // TODO: just remove from the list
                sender.input(SystemSelectMsg::FetchSystems);
            }
            CommandMsg::Deleted(Err(e)) => {
                eprintln!("Error deleting system: {:?}", e);
            }
        }
    }
}
