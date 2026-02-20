use std::sync::Arc;

use database::repository_manager::RepositoryManager;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{error::Error as ServiceError, view_models::SystemListModel};
use ui_components::confirm_dialog::{
    ConfirmDialog, ConfirmDialogInit, ConfirmDialogMsg, ConfirmDialogOutputMsg,
};

use crate::{
    list_item::DeletableListItem,
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
    SelectionChanged,
}

#[derive(Debug)]
pub enum SystemSelectOutputMsg {
    SystemSelected(SystemListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    SystemsFetched(Result<Vec<SystemListModel>, ServiceError>),
    Deleted {
        result: Result<(), ServiceError>,
        id: i64,
    },
}

pub struct SystemSelectInit {
    pub repository_manager: Arc<RepositoryManager>,
    pub app_services: Arc<service::app_services::AppServices>,
}

#[derive(Debug)]
pub struct SystemSelectModel {
    repository_manager: Arc<RepositoryManager>,
    app_services: Arc<service::app_services::AppServices>,
    list_view_wrapper: TypedListView<DeletableListItem, gtk::SingleSelection>,
    selected_system_ids: Vec<i64>,
    system_form_controller: Controller<SystemFormModel>,
    confirm_dialog_controller: Controller<ConfirmDialog>,
    selected_list_item: Option<DeletableListItem>,
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
                    gtk::Button {
                        set_label: "Delete",
                        connect_clicked => SystemSelectMsg::DeleteClicked,
                        #[watch]
                        set_sensitive: model.selected_list_item.as_ref().is_some_and(|item| item.can_delete),
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
        let list_view_wrapper: TypedListView<DeletableListItem, gtk::SingleSelection> =
            TypedListView::with_sorting();

        list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |_| {
                    sender.input(SystemSelectMsg::SelectionChanged);
                }
            ));

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
                app_services: Arc::clone(&init_model.app_services),
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
            repository_manager: init_model.repository_manager,
            app_services: init_model.app_services,
            list_view_wrapper,
            selected_system_ids: Vec::new(),
            system_form_controller,
            confirm_dialog_controller,
            selected_list_item: None,
        };

        let systems_list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();

        sender.input(SystemSelectMsg::FetchSystems);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            SystemSelectMsg::FetchSystems => {
                tracing::info!("Fetching systems.");
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    let systems_result = app_services.view_model.get_system_list_models().await;
                    CommandMsg::SystemsFetched(systems_result)
                });
            }
            SystemSelectMsg::SelectClicked => {
                if let Some(system) = self.get_selected_system_list_model() {
                    sender
                        .output(SystemSelectOutputMsg::SystemSelected(system.clone()))
                        .unwrap_or_else(|e| {
                            tracing::error!(error = ?e, "Failed to send system selection output")
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
                if let Some(edit_system) = self.get_selected_system_list_model() {
                    self.system_form_controller.emit(SystemFormMsg::Show {
                        edit_system: Some(edit_system.clone()),
                    });
                }
            }
            SystemSelectMsg::DeleteClicked => {
                self.confirm_dialog_controller.emit(ConfirmDialogMsg::Show);
            }
            SystemSelectMsg::DeleteConfirmed => {
                let app_services = Arc::clone(&self.app_services);
                if let Some(id) = self.get_selected_system_list_model().map(|item| item.id) {
                    sender.oneshot_command(async move {
                        tracing::info!(id = id, "Deleting system");
                        let result = app_services.system.delete_system(id).await;
                        CommandMsg::Deleted { result, id }
                    });
                }
            }
            SystemSelectMsg::SystemAdded(system_list_model) => {
                if !self.selected_system_ids.contains(&system_list_model.id) {
                    self.add_to_list(&system_list_model);
                }
            }
            SystemSelectMsg::SystemUpdated(system_list_model) => {
                tracing::info!(id = system_list_model.id, "Updating system list item");
                self.remove_from_list(system_list_model.id);
                self.add_to_list(&system_list_model);
            }
            SystemSelectMsg::Ignore => (),
            SystemSelectMsg::SelectionChanged => {
                self.selected_list_item = self.get_selected_list_item();
            }
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
                tracing::info!(len = systems.len(), "Fetched systems");
                self.populate_list(systems);
            }
            CommandMsg::SystemsFetched(Err(e)) => {
                tracing::error!(error = ?e, "Error fetching systems");
                show_error_dialog(format!("Error fetching systems: {:?}", e), root);
            }
            CommandMsg::Deleted { result, id } => match result {
                Ok(_) => {
                    tracing::info!("System deleted successfully");
                    self.remove_from_list(id);
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Error deleting system");
                    show_error_dialog(format!("Error deleting system ID {}: {:?}", id, e), root);
                }
            },
        }
    }
}

impl SystemSelectModel {
    fn remove_from_list(&mut self, id: i64) {
        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get(i)
                && item.borrow().id == id
            {
                tracing::info!(id = id, "Removing list item from list");
                self.list_view_wrapper.remove(i);
                break;
            }
        }
    }

    fn add_to_list(&mut self, system_list_model: &SystemListModel) {
        let new_item = DeletableListItem {
            name: system_list_model.name.clone(),
            id: system_list_model.id,
            can_delete: system_list_model.can_delete,
        };
        self.list_view_wrapper.append(new_item);

        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get_visible(i)
                && item.borrow().id == system_list_model.id
            {
                tracing::info!(
                    id = system_list_model.id,
                    "Selecting newly added system list item"
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
            .map(|system| DeletableListItem {
                name: system.name.clone(),
                id: system.id,
                can_delete: system.can_delete,
            });
        self.list_view_wrapper.clear();
        self.list_view_wrapper.extend_from_iter(list_items);
    }

    fn get_selected_list_item(&self) -> Option<DeletableListItem> {
        let selected_index = self.list_view_wrapper.selection_model.selected();
        if let Some(item) = self.list_view_wrapper.get_visible(selected_index) {
            let item = item.borrow();
            Some(item.clone())
        } else {
            None
        }
    }

    fn get_selected_system_list_model(&self) -> Option<SystemListModel> {
        if let Some(item) = self.get_selected_list_item() {
            Some(SystemListModel {
                id: item.id,
                name: item.name.clone(),
                can_delete: item.can_delete,
            })
        } else {
            None
        }
    }
}
