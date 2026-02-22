use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, glib,
        prelude::{
            ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, FrameExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
};
use service::{
    app_services::AppServices,
    error::Error as ServiceError,
    view_models::{FileSetListModel, ReleaseViewModel, SoftwareTitleListModel, SystemListModel},
};

use crate::{
    list_item::ListItem,
    release_form_components::{
        file_set_list::{FileSetList, FileSetListInit, FileSetListMsg, FileSetListOutputMsg},
        item_list::{ItemList, ItemListInit, ItemListMsg, ItemListOutputMsg},
        software_title_list::{
            SoftwareTitleList, SoftwareTitleListInit, SoftwareTitleListMsg,
            SoftwareTitleListOutputMsg,
        },
        system_list::{SystemList, SystemListInit, SystemListMsg, SystemListOutputMsg},
    },
    utils::dialog_utils::{show_error_dialog, show_info_dialog},
};

#[derive(Debug)]
pub enum ReleaseFormMsg {
    StartSaveRelease,
    Show { release_id: Option<i64> },
    Hide,
    NameChanged(String),
    UpdateEditFields,
    SystemsChanged { system_ids: Vec<i64> },
    FileSetsChanged { file_set_ids: Vec<i64> },
    SoftwareTitlesChanged { software_title_ids: Vec<i64> },
    SoftwareTitleCreated(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
    ItemsChanged { item_ids: Vec<i64> },
}

#[derive(Debug)]
pub enum ReleaseFormOutputMsg {
    ReleaseCreatedOrUpdated { id: i64 },
    SoftwareTitleCreated(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    ReleaseCreatedOrUpdated(Result<i64, ServiceError>),
    ReleaseFetched(Result<ReleaseViewModel, ServiceError>),
}

#[derive(Debug)]
pub struct ReleaseFormModel {
    app_services: Arc<AppServices>,

    release: Option<ReleaseViewModel>,
    release_name: String,

    selected_file_set_ids: Vec<i64>,
    file_set_list: Controller<FileSetList>,
    selected_system_ids: Vec<i64>,
    system_list: Controller<SystemList>,
    selected_software_title_ids: Vec<i64>,
    software_title_list: Controller<SoftwareTitleList>,
    selected_item_ids: Vec<i64>,
    item_list: Controller<ItemList>,
}

pub struct ReleaseFormInit {
    pub app_services: Arc<AppServices>,
}

#[relm4::component(pub)]
impl Component for ReleaseFormModel {
    type Input = ReleaseFormMsg;
    type Output = ReleaseFormOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = ReleaseFormInit;

    view! {
        #[root]
        gtk::Window {
            set_default_width: 800,
            set_default_height: 600,
            set_title: Some("Release Form"),
            set_margin_top: 5,
            set_margin_bottom: 5,
            set_margin_start: 5,
            set_margin_end: 5,

            connect_close_request[sender] => move |_| {
                sender.input(ReleaseFormMsg::Hide);
                glib::Propagation::Proceed
            },


            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                add_css_class: "form-container",

                gtk::Frame {
                    set_label: Some("Release Name:"),
                    #[name="release_name_entry"]
                    gtk::Entry {
                        set_text: &model.release_name,
                        set_placeholder_text: Some("Release name"),
                        connect_changed[sender] => move |entry| {
                            let buffer = entry.buffer();
                            sender.input(ReleaseFormMsg::NameChanged(buffer.text().into()));
                        },
                        set_hexpand: true,
                    },
                },

                #[name="notebook"]
                gtk::Notebook { },

                gtk::Button {
                    set_label: "Submit Release",
                    connect_clicked => ReleaseFormMsg::StartSaveRelease,
                },
            },
        }
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let file_set_list_init = FileSetListInit {
            app_services: Arc::clone(&init_model.app_services),
            selected_system_ids: vec![],
        };
        let file_set_list = FileSetList::builder().launch(file_set_list_init).forward(
            sender.input_sender(),
            |msg| match msg {
                FileSetListOutputMsg::ItemsChanged { file_set_ids } => {
                    ReleaseFormMsg::FileSetsChanged { file_set_ids }
                }
            },
        );

        let system_list_init = SystemListInit {
            app_services: Arc::clone(&init_model.app_services),
        };
        let system_list =
            SystemList::builder()
                .launch(system_list_init)
                .forward(sender.input_sender(), |msg| match msg {
                    SystemListOutputMsg::ItemsChanged { system_ids } => {
                        ReleaseFormMsg::SystemsChanged { system_ids }
                    }
                });

        let software_title_list_init = SoftwareTitleListInit {
            app_services: Arc::clone(&init_model.app_services),
        };
        let software_title_list = SoftwareTitleList::builder()
            .launch(software_title_list_init)
            .forward(sender.input_sender(), |msg| match msg {
                SoftwareTitleListOutputMsg::ItemsChanged { software_title_ids } => {
                    ReleaseFormMsg::SoftwareTitlesChanged { software_title_ids }
                }
                SoftwareTitleListOutputMsg::SoftwareTitleCreated(software_title) => {
                    ReleaseFormMsg::SoftwareTitleCreated(software_title)
                }
                SoftwareTitleListOutputMsg::SoftwareTitleUpdated(software_title) => {
                    ReleaseFormMsg::SoftwareTitleUpdated(software_title)
                }
            });

        let item_list_init = ItemListInit {
            release_id: None,
            app_services: Arc::clone(&init_model.app_services),
        };
        let item_list =
            ItemList::builder()
                .launch(item_list_init)
                .forward(sender.input_sender(), |msg| match msg {
                    ItemListOutputMsg::ItemsChanged { item_ids } => {
                        ReleaseFormMsg::ItemsChanged { item_ids }
                    }
                });

        let model = ReleaseFormModel {
            app_services: init_model.app_services,
            release: None,
            release_name: String::new(),
            file_set_list,
            selected_file_set_ids: vec![],
            system_list,
            selected_system_ids: vec![],
            software_title_list,
            selected_software_title_ids: vec![],
            item_list,
            selected_item_ids: vec![],
        };

        let file_set_list_view = model.file_set_list.widget();
        let system_list_view = model.system_list.widget();
        let software_title_list_view = model.software_title_list.widget();
        let item_list_view = model.item_list.widget();

        let widgets = view_output!();
        widgets.notebook.append_page(
            software_title_list_view,
            Some(&gtk::Label::new(Some("Software Titles"))),
        );
        widgets
            .notebook
            .append_page(system_list_view, Some(&gtk::Label::new(Some("Systems"))));
        widgets.notebook.append_page(
            file_set_list_view,
            Some(&gtk::Label::new(Some("File Sets"))),
        );
        widgets
            .notebook
            .append_page(item_list_view, Some(&gtk::Label::new(Some("Items"))));
        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            ReleaseFormMsg::SystemsChanged { system_ids } => {
                self.selected_system_ids = system_ids.clone();
                self.file_set_list
                    .emit(FileSetListMsg::SystemsChanged { system_ids });
            }
            ReleaseFormMsg::FileSetsChanged { file_set_ids } => {
                self.selected_file_set_ids = file_set_ids;
            }
            ReleaseFormMsg::SoftwareTitlesChanged { software_title_ids } => {
                self.selected_software_title_ids = software_title_ids;
            }
            ReleaseFormMsg::ItemsChanged { item_ids } => {
                self.selected_item_ids = item_ids;
            }
            ReleaseFormMsg::StartSaveRelease => {
                tracing::info!("Starting to save release with selected systems and file sets");
                let app_services = Arc::clone(&self.app_services);
                let software_title_ids = self.selected_software_title_ids.clone();
                let system_ids = self.selected_system_ids.clone();

                let file_set_ids = self.selected_file_set_ids.clone();

                tracing::info!(
                    system_ids = ?system_ids,
                    file_set_ids = ?file_set_ids,
                    software_title_ids = ?software_title_ids,
                    "Collected IDs for release"
                );

                if system_ids.is_empty() {
                    show_info_dialog(
                        "No systems selected, cannot create release.".to_string(),
                        root,
                    );
                } else if file_set_ids.is_empty() {
                    show_info_dialog(
                        "No file sets selected, cannot create release.".to_string(),
                        root,
                    );
                } else if software_title_ids.is_empty() {
                    show_info_dialog(
                        "No software titles selected, cannot create release.".to_string(),
                        root,
                    );
                } else {
                    let release_id = self.release.as_ref().map(|r| r.id);
                    let release_name = self.release_name.clone();

                    sender.oneshot_command(async move {
                        let res = match release_id {
                            Some(id) => {
                                tracing::info!(id = id, "Editing existing release");
                                app_services
                                    .release()
                                    .update_release(
                                        id,
                                        release_name.as_str(),
                                        &software_title_ids,
                                        &file_set_ids,
                                        &system_ids,
                                    )
                                    .await
                            }
                            _ => {
                                tracing::info!(name = release_name, "Creating new release");
                                app_services
                                    .release()
                                    .add_release(
                                        release_name.as_str(),
                                        &software_title_ids,
                                        &file_set_ids,
                                        &system_ids,
                                    )
                                    .await
                            }
                        };
                        CommandMsg::ReleaseCreatedOrUpdated(res)
                    });
                }
            }
            ReleaseFormMsg::SoftwareTitleCreated(software_title) => {
                tracing::info!(id = software_title.id, "Software title created");
                sender
                    .output(ReleaseFormOutputMsg::SoftwareTitleCreated(software_title))
                    .unwrap_or_else(
                        |err| tracing::error!(error = ?err, "Error in sending message"),
                    );
            }
            ReleaseFormMsg::SoftwareTitleUpdated(software_title) => {
                tracing::info!(id = software_title.id, "Software title updated");
                sender
                    .output(ReleaseFormOutputMsg::SoftwareTitleUpdated(software_title))
                    .unwrap_or_else(
                        |err| tracing::error!(error = ?err, "Error in sending message"),
                    );
            }
            ReleaseFormMsg::UpdateEditFields => {
                let mut selected_systems = vec![];
                let mut selected_file_sets = vec![];
                let mut selected_software_titles = vec![];
                let mut selected_items = vec![];
                let mut release_name = String::new();
                let release_id = self.release.as_ref().map(|r| r.id);

                if let Some(release) = &self.release {
                    selected_systems = release
                        .systems
                        .iter()
                        .map(|s| SystemListModel {
                            id: s.id,
                            name: s.name.clone(),
                            can_delete: false,
                        })
                        .collect();

                    selected_file_sets = release
                        .file_sets
                        .iter()
                        .map(|fs| FileSetListModel {
                            id: fs.id,
                            file_set_name: fs.file_set_name.clone(),
                            file_type: fs.file_type,
                            file_name: fs.file_name.clone(),
                            can_delete: fs.can_delete,
                        })
                        .collect();

                    selected_software_titles = release
                        .software_titles
                        .iter()
                        .map(|st| SoftwareTitleListModel {
                            id: st.id,
                            name: st.name.clone(),
                            can_delete: false,
                        })
                        .collect();

                    selected_items = release
                        .items
                        .iter()
                        .map(|item| ListItem {
                            id: item.id,
                            name: item.item_type.to_string(),
                        })
                        .collect();

                    release_name = release.name.clone();
                }

                widgets.release_name_entry.set_text(release_name.as_str());
                self.release_name = release_name;

                self.system_list.emit(SystemListMsg::ResetItems {
                    items: selected_systems.clone(),
                });
                self.selected_system_ids = selected_systems.iter().map(|s| s.id).collect();

                self.file_set_list.emit(FileSetListMsg::ResetItems {
                    items: selected_file_sets,
                    system_ids: selected_systems.iter().map(|s| s.id).collect(),
                });

                self.software_title_list
                    .emit(SoftwareTitleListMsg::ResetItems {
                        items: selected_software_titles.clone(),
                    });
                self.selected_software_title_ids =
                    selected_software_titles.iter().map(|st| st.id).collect();

                self.item_list
                    .emit(ItemListMsg::SetReleaseId { release_id });
                self.item_list.emit(ItemListMsg::ResetItems {
                    items: selected_items,
                });
            }
            ReleaseFormMsg::Show { release_id } => {
                if let Some(id) = release_id {
                    tracing::info!(id = id, "Loading release");
                    let app_services = Arc::clone(&self.app_services);
                    sender.oneshot_command(async move {
                        let release_result =
                            app_services.view_model().get_release_view_model(id).await;
                        CommandMsg::ReleaseFetched(release_result)
                    });
                } else {
                    self.release = None;
                    sender.input(ReleaseFormMsg::UpdateEditFields);
                }

                root.show();
            }
            ReleaseFormMsg::Hide => {
                root.hide();
            }
            ReleaseFormMsg::NameChanged(name) => {
                self.release_name = name;
            }
        }
        // This is essential with update_with_view:
        self.update_view(widgets, sender);
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            CommandMsg::ReleaseCreatedOrUpdated(Ok(id)) => {
                tracing::info!(id = id, "Release created or updated");
                sender
                    .output(ReleaseFormOutputMsg::ReleaseCreatedOrUpdated { id })
                    .unwrap_or_else(
                        |err| tracing::error!(error = ?err, "Error sending output message"),
                    );
                root.close();
            }
            CommandMsg::ReleaseCreatedOrUpdated(Err(err)) => {
                show_error_dialog(
                    format!("Failed to create or update release: {:?}", err),
                    root,
                );
            }
            CommandMsg::ReleaseFetched(Ok(release)) => {
                tracing::info!(id = release.id, "Release fetched");
                self.release = Some(release);
                sender.input(ReleaseFormMsg::UpdateEditFields);
            }
            CommandMsg::ReleaseFetched(Err(err)) => {
                tracing::error!(error = ?err, "Failed to fetch release");
                show_error_dialog(format!("Failed to fetch release: {:?}", err), root);
            }
        }
    }
}
