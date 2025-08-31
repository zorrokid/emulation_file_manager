use std::sync::Arc;

use database::{database_error::DatabaseError, repository_manager::RepositoryManager};
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
    view_models::SoftwareTitleListModel,
};

use crate::{
    list_item::ListItem,
    software_title_form::{
        SoftwareTitleFormInit, SoftwareTitleFormModel, SoftwareTitleFormMsg,
        SoftwareTitleFormOutputMsg,
    },
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
    Deleted(Result<i64, DatabaseError>),
    Canceled,
}

pub struct SoftwareTitleSelectInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    //pub selected_software_title_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct SoftwareTitleSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    software_titles: Vec<SoftwareTitleListModel>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_software_title_ids: Vec<i64>,
    software_title_form_controller: Controller<SoftwareTitleFormModel>,
    confirm_dialog_controller: Controller<ConfirmDialog>,
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
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::with_sorting();

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
                repository_manager: Arc::clone(&init_model.repository_manager),
                //edit_software_title,
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
            software_titles: Vec::new(),
            list_view_wrapper,
            selected_software_title_ids: Vec::new(),
            software_title_form_controller,
            confirm_dialog_controller,
        };

        let software_titles_list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();

        sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            SoftwareTitleSelectMsg::FetchSoftwareTitles => {
                println!("Fetching software_titles...");
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let software_titles_result =
                        view_model_service.get_software_title_list_models().await;
                    CommandMsg::SoftwareTitlesFetched(software_titles_result)
                });
            }
            SoftwareTitleSelectMsg::SelectClicked => {
                let selected = self.list_view_wrapper.selection_model.selected();
                if let Some(software_title) = self.list_view_wrapper.get_visible(selected) {
                    let software_title = software_title.borrow();
                    println!(
                        "SoftwareTitle selected: {} with ID: {}",
                        software_title.name, software_title.id
                    );
                    let res = sender.output(SoftwareTitleSelectOutputMsg::Selected(
                        SoftwareTitleListModel {
                            id: software_title.id,
                            name: software_title.name.clone(),
                            can_delete: false, // TODO
                        },
                    ));
                    match res {
                        Ok(_) => {
                            println!("SoftwareTitle selection output sent successfully.");
                            sender.input(SoftwareTitleSelectMsg::Hide);
                        }
                        Err(e) => {
                            eprintln!("Failed to send software_title selection output: {:?}", e)
                        }
                    }
                } else {
                    eprintln!("No software_title found at selected index {}", selected);
                }
            }
            SoftwareTitleSelectMsg::AddClicked => {
                self.software_title_form_controller
                    .emit(SoftwareTitleFormMsg::Show {
                        edit_software_title: None,
                    });
            }
            SoftwareTitleSelectMsg::EditClicked => {
                let edit_software_title = self
                    .list_view_wrapper
                    .get_visible(self.list_view_wrapper.selection_model.selected())
                    .and_then(|st| {
                        self.software_titles
                            .iter()
                            .find(|s| s.id == st.borrow().id)
                            .cloned()
                    });
                if let Some(edit_software_title) = &edit_software_title {
                    self.software_title_form_controller
                        .emit(SoftwareTitleFormMsg::Show {
                            edit_software_title: Some(edit_software_title.clone()),
                        });
                }
            }
            SoftwareTitleSelectMsg::DeleteClicked => {
                println!("Delete clicked");
                self.confirm_dialog_controller.emit(ConfirmDialogMsg::Show);
            }
            SoftwareTitleSelectMsg::DeleteCanceled => {
                println!("Canceled deletion");
            }
            SoftwareTitleSelectMsg::DeleteConfirmed => {
                let repository_manager = Arc::clone(&self.repository_manager);
                let selected = self.list_view_wrapper.selection_model.selected();
                let software_title = self.list_view_wrapper.get_visible(selected);
                let id = software_title.as_ref().map(|st| st.borrow().id);
                if let Some(id) = id {
                    sender.oneshot_command(async move {
                        println!("Deleting software_title with ID {}", id);
                        let result = repository_manager
                            .get_software_title_repository()
                            .delete_software_title(id)
                            .await;
                        CommandMsg::Deleted(result)
                    });
                }
            }
            SoftwareTitleSelectMsg::SoftwareTitleAdded(software_title_list_model) => {
                println!(
                    "Successfully added software_title: {}",
                    software_title_list_model.name
                );
                sender
                    .output(SoftwareTitleSelectOutputMsg::Created(
                        software_title_list_model,
                    ))
                    .expect("Failed to send software_title selection output");
                sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
            }
            SoftwareTitleSelectMsg::SoftwareTitleUpdated(_software_title_list_model) => {
                sender
                    .output(SoftwareTitleSelectOutputMsg::Updated(
                        _software_title_list_model,
                    ))
                    .expect("Failed to send software_title update output");
                sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
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
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            CommandMsg::SoftwareTitlesFetched(Ok(software_titles)) => {
                self.software_titles = software_titles;
                let list_items = self
                    .software_titles
                    .iter()
                    .filter(|st| !self.selected_software_title_ids.contains(&st.id))
                    .map(|software_title| ListItem {
                        name: software_title.name.clone(),
                        id: software_title.id,
                    });
                self.list_view_wrapper.clear();
                self.list_view_wrapper.extend_from_iter(list_items);
            }
            CommandMsg::SoftwareTitlesFetched(Err(e)) => {
                eprintln!("Error fetching software_titles: {:?}", e);
                // TODO: show error to user
            }
            CommandMsg::Deleted(Ok(_id)) => {
                println!("Successfully deleted software_title");
                sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
            }
            CommandMsg::Deleted(Err(error)) => {
                eprintln!("Error deleting software_title: {:?}", error);
            }
            CommandMsg::Canceled => {
                // No action needed
            }
        }
    }
}
