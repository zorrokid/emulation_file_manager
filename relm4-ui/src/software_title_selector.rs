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
    view_models::SoftwareTitleListModel,
};

use crate::list_item::ListItem;

#[derive(Debug)]
pub enum SoftwareTitleSelectMsg {
    FetchSoftwareTitles,
    AddSoftwareTitle { name: String },
    SelectClicked,
}

#[derive(Debug)]
pub enum SoftwareTitleSelectOutputMsg {
    SoftwareTitleSelected(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    SoftwareTitlesFetched(Result<Vec<SoftwareTitleListModel>, ServiceError>),
    SoftwareTitleAdded(SoftwareTitleListModel),
    AddingSoftwareTitleFailed(DatabaseError),
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
            set_default_width: 800,
            set_default_height: 800,
            set_title: Some("SoftwareTitle Selector"),
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Entry {
                    connect_activate[sender] => move |entry|  {
                        let buffer = entry.buffer();
                        sender.input(
                            SoftwareTitleSelectMsg::AddSoftwareTitle {
                                name: buffer.text().into(),
                            }
                        );
                        buffer.delete_text(0, None);
                    }
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    software_titles_list_view -> gtk::ListView {}
                },

                gtk::Button {
                    set_label: "Select SoftwareTitle",
                    connect_clicked => SoftwareTitleSelectMsg::SelectClicked,
                },

            }
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> = TypedListView::new();

        let model = SoftwareTitleSelectModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            software_titles: Vec::new(),
            list_view_wrapper,
            selected_software_title_ids: init_model.selected_software_title_ids,
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
            SoftwareTitleSelectMsg::AddSoftwareTitle { name } => {
                println!("Adding new software_title: {}", name);
                let repository_manager = Arc::clone(&self.repository_manager);
                sender.oneshot_command(async move {
                    let result = repository_manager
                        .get_software_title_repository()
                        .add_software_title(&name, None) // TODO: franchise_id
                        .await;
                    match result {
                        Ok(id) => {
                            let software_title_list_model = SoftwareTitleListModel {
                                id,
                                name: name.clone(),
                                can_delete: true, // OK to delete since this was just added
                            };
                            CommandMsg::SoftwareTitleAdded(software_title_list_model)
                        }
                        Err(e) => CommandMsg::AddingSoftwareTitleFailed(e),
                    }
                });
            }
            SoftwareTitleSelectMsg::SelectClicked => {
                let selected = self.list_view_wrapper.selection_model.selected();
                if let Some(software_title) = self.list_view_wrapper.get(selected) {
                    let software_title = software_title.borrow();
                    println!(
                        "SoftwareTitle selected: {} with ID: {}",
                        software_title.name, software_title.id
                    );
                    let res = sender.output(SoftwareTitleSelectOutputMsg::SoftwareTitleSelected(
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
                self.list_view_wrapper.extend_from_iter(list_items);
            }
            CommandMsg::SoftwareTitlesFetched(Err(e)) => {
                eprintln!("Error fetching software_titles: {:?}", e);
                // TODO: show error to user
            }
            CommandMsg::SoftwareTitleAdded(software_title_list_model) => {
                println!(
                    "Successfully added software_title: {}",
                    software_title_list_model.name
                );
                sender.input(SoftwareTitleSelectMsg::FetchSoftwareTitles);
            }
            CommandMsg::AddingSoftwareTitleFailed(error) => {
                eprintln!("Error adding software_title: {:?}", error);
                // TODO: show error to user
            }
        }
    }
}
