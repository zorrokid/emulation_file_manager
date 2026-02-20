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
use service::{
    error::Error as ServiceError, view_model_service::ViewModelService,
    view_models::SoftwareTitleListModel,
};

use crate::{
    list_item::DeletableListItem,
    software_title_form::{
        SoftwareTitleFormInit, SoftwareTitleFormModel, SoftwareTitleFormMsg,
        SoftwareTitleFormOutputMsg,
    },
    utils::dialog_utils::show_error_dialog,
};

use ui_components::confirm_dialog::{
    ConfirmDialog, ConfirmDialogInit, ConfirmDialogMsg, ConfirmDialogOutputMsg,
};

#[derive(Debug)]
pub enum SoftwareTitleSelectMsg {
    FetchSoftwareTitles,
    SelectClicked,
    EditClicked,
    AddClicked,
    DeleteClicked,
    SoftwareTitleAdded(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
    DeleteConfirmed,
    DeleteCanceled,
    Show {
        selected_software_title_ids: Vec<i64>,
    },
    Hide,
    SelectionChanged,
}

#[derive(Debug)]
pub enum SoftwareTitleSelectOutputMsg {
    Selected(SoftwareTitleListModel),
    Created(SoftwareTitleListModel),
    Updated(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, ServiceError>),
    Deleted(Result<i64, ServiceError>),
}

pub struct SoftwareTitleSelectInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub app_services: Arc<service::app_services::AppServices>,
}

#[derive(Debug)]
pub struct SoftwareTitleSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    app_services: Arc<service::app_services::AppServices>,
    software_titles: Vec<SoftwareTitleListModel>,
    list_view_wrapper: TypedListView<DeletableListItem, gtk::SingleSelection>,
    selected_software_title_ids: Vec<i64>,
    software_title_form_controller: Controller<SoftwareTitleFormModel>,
    confirm_dialog_controller: Controller<ConfirmDialog>,
    selected_list_item: Option<DeletableListItem>,
}

#[relm4::component(pub)]
impl Component for SoftwareTitleSelectModel {
    type Input = SoftwareTitleSelectMsg;
    type Output = SoftwareTitleSelectOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = SoftwareTitleSelectInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 400,
            set_default_height: 600,
            set_title: Some("SoftwareTitle Selector"),

            connect_close_request[sender] => move |_| {
                sender.input(SoftwareTitleSelectMsg::Hide);
                glib::Propagation::Proceed
            },
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    software_titles_list_view -> gtk::ListView {}
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,

                    gtk::Button {
                        set_label: "Add",
                        connect_clicked => SoftwareTitleSelectMsg::AddClicked,
                    },
                    gtk::Button {
                        set_label: "Edit",
                        connect_clicked => SoftwareTitleSelectMsg::EditClicked,
                    },
                    gtk::Button {
                        set_label: "Delete",
                        connect_clicked => SoftwareTitleSelectMsg::DeleteClicked,
                        #[watch]
                        set_sensitive: model.selected_list_item.as_ref().is_some_and(|item| item.can_delete),
                    },
                    gtk::Button {
                        set_label: "Select",
                        connect_clicked => SoftwareTitleSelectMsg::SelectClicked,
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
                    sender.input(SoftwareTitleSelectMsg::SelectionChanged);
                }
            ));

        let confirm_dialog_controller = ConfirmDialog::builder()
            .transient_for(&root)
            .launch(ConfirmDialogInit {
                title: "Confirm Deletion".to_string(),
                visible: false,
            })
            .forward(sender.input_sender(), |msg| match msg {
                ConfirmDialogOutputMsg::Confirmed => SoftwareTitleSelectMsg::DeleteConfirmed,
                ConfirmDialogOutputMsg::Canceled => SoftwareTitleSelectMsg::DeleteCanceled,
            });

        let software_title_form_controller = SoftwareTitleFormModel::builder()
            .transient_for(&root)
            .launch(SoftwareTitleFormInit {
                app_services: Arc::clone(&init_model.app_services),
            })
            .forward(sender.input_sender(), |msg| match msg {
                SoftwareTitleFormOutputMsg::SoftwareTitleAdded(software_title_list_model) => {
                    SoftwareTitleSelectMsg::SoftwareTitleAdded(software_title_list_model)
                }
                SoftwareTitleFormOutputMsg::SoftwareTitleUpdated(software_title_list_model) => {
                    SoftwareTitleSelectMsg::SoftwareTitleUpdated(software_title_list_model)
                }
            });

        let model = SoftwareTitleSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            app_services: init_model.app_services,
            software_titles: Vec::new(),
            list_view_wrapper,
            selected_software_title_ids: Vec::new(),
            software_title_form_controller,
            confirm_dialog_controller,
            selected_list_item: None,
        };

        let software_titles_list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();

        sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            SoftwareTitleSelectMsg::FetchSoftwareTitles => {
                tracing::info!("Fetching software_titles.");
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    let software_titles_result = app_services
                        .view_model
                        .get_software_title_list_models()
                        .await;
                    CommandMsg::SoftwareTitlesFetched(software_titles_result)
                });
            }
            SoftwareTitleSelectMsg::SelectClicked => {
                if let Some(software_title) = self.get_selected_software_title_list_model() {
                    sender
                        .output(SoftwareTitleSelectOutputMsg::Selected(software_title))
                        .unwrap_or_else(|e| {
                            tracing::error!(
                                error = ?e,
                                "Failed to send software_title selection output"
                            );
                        });

                    sender.input(SoftwareTitleSelectMsg::Hide);
                }
            }
            SoftwareTitleSelectMsg::AddClicked => {
                self.software_title_form_controller
                    .emit(SoftwareTitleFormMsg::Show {
                        edit_software_title: None,
                    });
            }
            SoftwareTitleSelectMsg::EditClicked => {
                if let Some(edit_software_title) = self.get_selected_software_title_list_model() {
                    self.software_title_form_controller
                        .emit(SoftwareTitleFormMsg::Show {
                            edit_software_title: Some(edit_software_title.clone()),
                        });
                }
            }
            SoftwareTitleSelectMsg::DeleteClicked => {
                self.confirm_dialog_controller.emit(ConfirmDialogMsg::Show);
            }
            SoftwareTitleSelectMsg::DeleteCanceled => {
                tracing::info!("Deletion canceled by user.");
            }
            SoftwareTitleSelectMsg::DeleteConfirmed => {
                let app_services = Arc::clone(&self.app_services);
                if let Some(id) = self.get_selected_list_item().map(|item| item.id) {
                    sender.oneshot_command(async move {
                        tracing::info!(id = id, "Deleting software_title");
                        let result = app_services.software_title.delete_software_title(id).await;
                        CommandMsg::Deleted(result)
                    });
                }
            }
            SoftwareTitleSelectMsg::SoftwareTitleAdded(software_title_list_model) => {
                tracing::info!(id = software_title_list_model.id, "Added software_title");
                if !self
                    .selected_software_title_ids
                    .contains(&software_title_list_model.id)
                {
                    self.add_to_list(&software_title_list_model);
                }

                sender
                    .output(SoftwareTitleSelectOutputMsg::Created(
                        software_title_list_model,
                    ))
                    .unwrap_or_else(|e| {
                        tracing::error!(error = ?e, "Failed to send software_title creation output");
                    });
            }
            SoftwareTitleSelectMsg::SoftwareTitleUpdated(software_title_list_model) => {
                self.remove_from_list(software_title_list_model.id);
                self.add_to_list(&software_title_list_model);
                sender
                    .output(SoftwareTitleSelectOutputMsg::Updated(
                        software_title_list_model,
                    ))
                    .unwrap_or_else(|_e| {
                        tracing::error!("Failed to send software_title update output");
                    });
            }
            SoftwareTitleSelectMsg::Show {
                selected_software_title_ids,
            } => {
                self.selected_software_title_ids = selected_software_title_ids;
                sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
                root.show();
            }
            SoftwareTitleSelectMsg::Hide => {
                root.hide();
            }
            SoftwareTitleSelectMsg::SelectionChanged => {
                self.selected_list_item = self.get_selected_list_item();
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::SoftwareTitlesFetched(Ok(software_titles)) => {
                self.software_titles = software_titles;
                self.populate_list(self.software_titles.clone());
            }
            CommandMsg::SoftwareTitlesFetched(Err(e)) => {
                let message = format!("Error fetching software titles: {}", e);
                tracing::error!(message);
                show_error_dialog(message, root);
            }
            CommandMsg::Deleted(Ok(_id)) => {
                tracing::info!("Software title deleted successfully.");
                sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
            }
            CommandMsg::Deleted(Err(e)) => {
                tracing::error!(error = ?e, "Error deleting software title");
                show_error_dialog(format!("Error deleting software title: {}", e), root);
            }
        }
    }
}

impl SoftwareTitleSelectModel {
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

    fn add_to_list(&mut self, list_model: &SoftwareTitleListModel) {
        let new_item = DeletableListItem {
            name: list_model.name.clone(),
            id: list_model.id,
            can_delete: list_model.can_delete,
        };
        self.list_view_wrapper.append(new_item);

        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get_visible(i)
                && item.borrow().id == list_model.id
            {
                tracing::info!(id = list_model.id, "Selecting newly added list item");
                self.list_view_wrapper.selection_model.set_selected(i);
                break;
            }
        }
    }

    fn populate_list(&mut self, software_titles: Vec<SoftwareTitleListModel>) {
        let list_items = software_titles
            .iter()
            .filter(|f| !self.selected_software_title_ids.contains(&f.id))
            .map(|software_title| DeletableListItem {
                name: software_title.name.clone(),
                id: software_title.id,
                can_delete: software_title.can_delete,
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

    fn get_selected_software_title_list_model(&self) -> Option<SoftwareTitleListModel> {
        if let Some(selected_item) = self.get_selected_list_item() {
            Some(SoftwareTitleListModel {
                id: selected_item.id,
                name: selected_item.name,
                can_delete: selected_item.can_delete,
            })
        } else {
            None
        }
    }
}
