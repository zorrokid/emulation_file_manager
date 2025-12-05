use std::sync::Arc;

use database::{database_error::Error as DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self, glib,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
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
    utils::dialog_utils::show_error_dialog,
};

#[derive(Debug)]
pub enum SystemSelectMsg {
    FetchSystems,
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
    Deleted {
        result: Result<(), DatabaseError>,
        id: i64,
    },
}

pub struct SystemSelectInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
}

#[derive(Debug)]
pub struct SystemSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
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
                    // TODO: Disable if system cannot be deleted
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
                tracing::info!("Fetching systems...");
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let systems_result = view_model_service.get_system_list_models().await;
                    CommandMsg::SystemsFetched(systems_result)
                });
            }
            SystemSelectMsg::SelectClicked => {
                if let Some(system) = self.get_selected_system() {
                    sender
                        .output(SystemSelectOutputMsg::SystemSelected(system.clone()))
                        .unwrap_or_else(|e| {
                            tracing::error!("Failed to send system selection output: {:?}", e)
                        });
                    root.close();
                }
            }
            SystemSelectMsg::Show {
                selected_system_ids,
            } => {
                self.selected_system_ids = selected_system_ids;
                sender.input(SystemSelectMsg::FetchSystems);
                root.show();
            }
            SystemSelectMsg::Hide => root.hide(),
            SystemSelectMsg::AddClicked => {
                self.system_form_controller
                    .emit(SystemFormMsg::Show { edit_system: None });
            }
            SystemSelectMsg::EditClicked => {
                if let Some(edit_system) = self.get_selected_system() {
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
                if let Some(system) = self.get_selected_system() {
                    let id = system.id;
                    sender.oneshot_command(async move {
                        tracing::info!("Deleting system with ID {}", id);
                        let result = repository_manager
                            .get_system_repository()
                            .delete_system(id)
                            .await;
                        CommandMsg::Deleted { result, id }
                    });
                }
            }
            SystemSelectMsg::SystemAdded(system_list_model) => {
                // TODO: is this check necessary here?
                if !self.selected_system_ids.contains(&system_list_model.id) {
                    self.add_system_to_list(&system_list_model);
                }
            }
            SystemSelectMsg::SystemUpdated(system_list_model) => {
                tracing::info!("Updating system list item ID {}", system_list_model.id);
                self.remove_item_from_list(system_list_model.id);
                self.add_system_to_list(&system_list_model);
            }
            SystemSelectMsg::Ignore => (),
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::SystemsFetched(Ok(systems)) => {
                tracing::info!("Fetched {} systems", systems.len());
                self.populate_list(systems);
            }
            CommandMsg::SystemsFetched(Err(e)) => {
                show_error_dialog(format!("Error fetching systems: {:?}", e), root);
            }
            CommandMsg::Deleted { result, id } => match result {
                Ok(_) => {
                    tracing::info!("System deleted successfully");
                    self.remove_item_from_list(id);
                }
                Err(e) => {
                    show_error_dialog(format!("Error deleting system: {:?}", e), root);
                }
            },
        }
    }
}

impl SystemSelectModel {
    fn remove_item_from_list(&mut self, id: i64) {
        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get(i)
                && item.borrow().id == id
            {
                tracing::info!("Removing system list item ID {} from list", id);
                self.list_view_wrapper.remove(i);
                break;
            }
        }
    }

    fn add_system_to_list(&mut self, system_list_model: &SystemListModel) {
        let new_item = ListItem {
            name: system_list_model.name.clone(),
            id: system_list_model.id,
        };
        self.list_view_wrapper.append(new_item);

        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get_visible(i)
                && item.borrow().id == system_list_model.id
            {
                tracing::info!(
                    "Selecting newly added system list item ID {} at index {}",
                    system_list_model.id,
                    i
                );
                self.list_view_wrapper.selection_model.set_selected(i);
                break;
            }
        }
    }

    fn populate_list(&mut self, systems: Vec<SystemListModel>) {
        let list_items = systems
            .iter()
            .filter(|f| !self.selected_system_ids.contains(&f.id))
            .map(|system| ListItem {
                name: system.name.clone(),
                id: system.id,
            });
        self.list_view_wrapper.clear();
        self.list_view_wrapper.extend_from_iter(list_items);
    }

    fn get_selected_system(&self) -> Option<SystemListModel> {
        let selected_index = self.list_view_wrapper.selection_model.selected();
        if let Some(item) = self.list_view_wrapper.get_visible(selected_index) {
            let item = item.borrow();
            Some(SystemListModel {
                id: item.id,
                name: item.name.clone(),
                can_delete: false, // TODO: add to list model
            })
        } else {
            None
        }
    }
}
