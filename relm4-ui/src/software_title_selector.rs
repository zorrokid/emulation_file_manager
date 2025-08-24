use std::sync::Arc;

use database::{database_error::DatabaseError, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
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
        SoftwareTitleFormInit, SoftwareTitleFormModel, SoftwareTitleFormOutputMsg,
    },
};

use ui_components::confirm_dialog::{ConfirmDialog, ConfirmDialogInit};

#[derive(Debug)]
pub enum SoftwareTitleSelectMsg {
    FetchSoftwareTitles,
    SelectClicked,
    EditClicked,
    AddClicked,
    DeleteClicked,
    SoftwareTitleAdded(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
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
    pub selected_software_title_ids: Vec<i64>,
}

#[derive(Debug)]
pub struct SoftwareTitleSelectModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    software_titles: Vec<SoftwareTitleListModel>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_software_title_ids: Vec<i64>,
    software_title_form_controller: Option<Controller<SoftwareTitleFormModel>>,
}

impl SoftwareTitleSelectModel {
    fn open_software_title_form(
        &mut self,
        sender: &ComponentSender<Self>,
        root: &gtk::Window,
        edit_software_title: Option<SoftwareTitleListModel>,
    ) {
        let software_title_form = SoftwareTitleFormModel::builder()
            .transient_for(root)
            .launch(SoftwareTitleFormInit {
                repository_manager: Arc::clone(&self.repository_manager),
                edit_software_title,
            })
            .forward(sender.input_sender(), |msg| match msg {
                SoftwareTitleFormOutputMsg::SoftwareTitleAdded(software_title_list_model) => {
                    SoftwareTitleSelectMsg::SoftwareTitleAdded(software_title_list_model)
                }
                SoftwareTitleFormOutputMsg::SoftwareTitleUpdated(software_title_list_model) => {
                    SoftwareTitleSelectMsg::SoftwareTitleUpdated(software_title_list_model)
                }
            });
        self.software_title_form_controller = Some(software_title_form);
        self.software_title_form_controller
            .as_ref()
            .expect("SoftwareTitle form controller should be initialized")
            .widget()
            .present();
    }
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

        let model = SoftwareTitleSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            software_titles: Vec::new(),
            list_view_wrapper,
            selected_software_title_ids: init_model.selected_software_title_ids,
            software_title_form_controller: None,
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
                            root.close();
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
                self.open_software_title_form(&sender, root, None);
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
                    self.open_software_title_form(&sender, root, Some(edit_software_title.clone()));
                }
            }
            SoftwareTitleSelectMsg::DeleteClicked => {
                let repository_manager = Arc::clone(&self.repository_manager);
                let selected = self.list_view_wrapper.selection_model.selected();
                let software_title = self.list_view_wrapper.get_visible(selected);
                let id = software_title.as_ref().map(|st| st.borrow().id);
                if let Some(id) = id {
                    let stream = ConfirmDialog::builder()
                        .transient_for(root)
                        .launch(ConfirmDialogInit {
                            title: "Confirm Deletion".to_string(),
                        })
                        .into_stream();
                    sender.oneshot_command(async move {
                        let result = stream.recv_one().await;

                        if let Some(will_delete) = result {
                            if will_delete {
                                println!("Deleting software_title with ID {}", id);
                                let result = repository_manager
                                    .get_software_title_repository()
                                    .delete_software_title(id)
                                    .await;
                                CommandMsg::Deleted(result)
                            } else {
                                println!("User canceled deletion");
                                CommandMsg::Canceled
                            }
                        } else {
                            println!("Dialog was closed without confirmation");
                            CommandMsg::Canceled
                        }
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
                // Handle the fetched software_titles, e.g., populate a dropdown or list
                for software_title in &software_titles {
                    println!(
                        "Fetched software_title: {} with ID: {}",
                        software_title.name, software_title.id
                    );
                }
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
