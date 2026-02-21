use std::sync::Arc;

use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error,
    view_model_service::ReleaseFilter,
    view_models::{ReleaseListModel, Settings, SoftwareTitleListModel},
};

use crate::{
    list_item::ListItem,
    release_form::{ReleaseFormInit, ReleaseFormModel, ReleaseFormMsg, ReleaseFormOutputMsg},
};

#[derive(Debug)]
pub enum ReleasesMsg {
    SoftwareTitleSelected { id: i64 },
    SoftwareTitleDeselected { id: i64 },
    ReleaseSelected,
    StartAddRelease,
    AddRelease(ReleaseListModel),
    FetchReleases,
    ReleaseCreatedOrUpdated { id: i64 },
    SofwareTitleCreated(SoftwareTitleListModel),
    SofwareTitleUpdated(SoftwareTitleListModel),
    RemoveRelease,
    EditRelease,
    Clear,
}

#[derive(Debug)]
pub enum CommandMsg {
    FetchedReleases(Result<Vec<ReleaseListModel>, Error>),
    ReleaseDeleted(Result<i64, Error>),
}

#[derive(Debug)]
pub struct ReleasesModel {
    app_services: Arc<service::app_services::AppServices>,
    release_form: Controller<ReleaseFormModel>,
    releases_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_software_title_ids: Vec<i64>,
}

pub struct ReleasesInit {
    pub app_services: Arc<service::app_services::AppServices>,
    pub settings: Arc<Settings>,
}

#[derive(Debug)]
pub enum ReleasesOutputMsg {
    SoftwareTitleCreated {
        software_title_list_model: SoftwareTitleListModel,
    },
    SoftwareTitleUpdated {
        software_title_list_model: SoftwareTitleListModel,
    },
    ReleaseSelected {
        id: i64,
    },
    ShowError(String),
}

#[relm4::component(pub)]
impl Component for ReleasesModel {
    type Input = ReleasesMsg;
    type Output = ReleasesOutputMsg;
    type CommandOutput = CommandMsg;
    type Init = ReleasesInit;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            set_margin_all: 10,

            gtk::Label {
                set_label: "Releases",
            },

            gtk::ScrolledWindow {
                set_vexpand: true,
                #[local_ref]
                releases_list_view -> gtk::ListView {}
            },

            gtk::Button {
                set_label: "Edit Release",
                connect_clicked => ReleasesMsg::EditRelease,
            },

            gtk::Button {
                set_label: "Remove Release",
                connect_clicked => ReleasesMsg::RemoveRelease,
            },

            gtk::Button {
                set_label: "Add Release",
                connect_clicked => ReleasesMsg::StartAddRelease,
            },

        },
    }

    fn init(
        init_model: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let release_form_init_model = ReleaseFormInit {
            app_services: Arc::clone(&init_model.app_services),
            settings: Arc::clone(&init_model.settings),
        };

        let release_form = ReleaseFormModel::builder()
            .transient_for(&root)
            .launch(release_form_init_model)
            .forward(sender.input_sender(), |msg| match msg {
                ReleaseFormOutputMsg::ReleaseCreatedOrUpdated { id } => {
                    tracing::info!(id = id, "Release created or updated");
                    ReleasesMsg::ReleaseCreatedOrUpdated { id }
                }
                ReleaseFormOutputMsg::SoftwareTitleCreated(software_title_list_model) => {
                    tracing::info!(id = software_title_list_model.id, "Software title created");
                    ReleasesMsg::SofwareTitleCreated(software_title_list_model)
                }
                ReleaseFormOutputMsg::SoftwareTitleUpdated(software_title_list_model) => {
                    tracing::info!(id = software_title_list_model.id, "Software title updated");
                    ReleasesMsg::SofwareTitleUpdated(software_title_list_model)
                }
            });

        let model = ReleasesModel {
            app_services: init_model.app_services,
            release_form,
            releases_list_view_wrapper: TypedListView::new(),
            selected_software_title_ids: vec![],
        };
        let releases_list_view = &model.releases_list_view_wrapper.view;
        let selection_model = &model.releases_list_view_wrapper.selection_model;
        selection_model.connect_selected_notify(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(ReleasesMsg::ReleaseSelected);
            }
        ));

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ReleasesMsg::SoftwareTitleSelected { id } => {
                tracing::info!(id = id, "Software title selected");
                self.selected_software_title_ids.push(id);
                sender.input(ReleasesMsg::FetchReleases);
            }
            ReleasesMsg::SoftwareTitleDeselected { id } => {
                tracing::info!(id = id, "Software title deselected");
                self.selected_software_title_ids.retain(|&x| x != id);
                sender.input(ReleasesMsg::FetchReleases);
            }
            ReleasesMsg::Clear => {
                tracing::info!("Clearing selected software titles");
                self.selected_software_title_ids.clear();
                self.releases_list_view_wrapper.clear();
            }
            ReleasesMsg::FetchReleases => {
                tracing::info!(
                    ids = ?self.selected_software_title_ids,
                    "Fetching releases for software title",
                );

                let app_services = Arc::clone(&self.app_services);
                let software_title_ids = self.selected_software_title_ids.clone();
                sender.oneshot_command(async move {
                    let releases_result = app_services
                        .view_model
                        .get_release_list_models(ReleaseFilter {
                            software_title_ids,
                            system_id: None,
                            file_set_id: None,
                        })
                        .await;
                    CommandMsg::FetchedReleases(releases_result)
                });
            }
            ReleasesMsg::ReleaseSelected => {
                if let Some(selected_id) = self.get_selected_release_id() {
                    sender
                        .output(ReleasesOutputMsg::ReleaseSelected { id: selected_id })
                        .unwrap_or_else(|err| {
                            tracing::error!(
                                error = ?err,
                                "Error sending ReleaseSelected message");
                        });
                }
            }

            ReleasesMsg::StartAddRelease => {
                self.release_form
                    .emit(ReleaseFormMsg::Show { release_id: None });
            }
            ReleasesMsg::AddRelease(release_list_model) => {
                tracing::info!(id = release_list_model.id, "Release added");
                self.releases_list_view_wrapper.append(ListItem {
                    id: release_list_model.id,
                    name: release_list_model.name,
                });
            }
            ReleasesMsg::ReleaseCreatedOrUpdated { id } => {
                tracing::info!(id = id, "Release created or updated");
                // TODO fetch only the created of updated release, or maybe the message would
                // include the required data to update the list
                sender.input(ReleasesMsg::FetchReleases);
            }
            ReleasesMsg::SofwareTitleCreated(software_title_list_model) => {
                sender
                    .output(ReleasesOutputMsg::SoftwareTitleCreated {
                        software_title_list_model,
                    })
                    .unwrap_or_else(|err| {
                        tracing::error!(
                            error = ?err,
                            "Error sending SoftwareTitleCreated message",
                        );
                    });
            }
            ReleasesMsg::SofwareTitleUpdated(software_title_list_model) => {
                sender
                    .output(ReleasesOutputMsg::SoftwareTitleUpdated {
                        software_title_list_model,
                    })
                    .unwrap_or_else(|err| {
                        tracing::error!(
                            error = ?err,
                            "Error sending SoftwareTitleUpdated message");
                    });
            }
            ReleasesMsg::RemoveRelease => {
                if let Some(release_id) = self.get_selected_release_id() {
                    let app_services = Arc::clone(&self.app_services);
                    sender.oneshot_command(async move {
                        let result = app_services.release.delete_release(release_id).await;
                        CommandMsg::ReleaseDeleted(result)
                    });
                }
            }
            ReleasesMsg::EditRelease => {
                if let Some(release_id) = self.get_selected_release_id() {
                    self.release_form.emit(ReleaseFormMsg::Show {
                        release_id: Some(release_id),
                    });
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
            CommandMsg::FetchedReleases(releases_result) => match releases_result {
                Ok(releases) => {
                    tracing::info!("Releases fetched successfully.");
                    let items: Vec<ListItem> = releases
                        .into_iter()
                        .map(|release| {
                            let parts: Vec<String> = vec![
                                release.name.clone(),
                                release.system_names.join(", "),
                                release.media_file_types.join(", "),
                            ]
                            .into_iter()
                            .filter(|s| !s.is_empty())
                            .collect();
                            let name = parts.join(" ");

                            ListItem {
                                id: release.id,
                                name,
                            }
                        })
                        .collect();
                    self.releases_list_view_wrapper.clear();
                    self.releases_list_view_wrapper.extend_from_iter(items);
                    sender.input(ReleasesMsg::ReleaseSelected);
                }
                Err(err) => {
                    tracing::error!(error = ?err, "Error fetching releases");
                    sender
                        .output(ReleasesOutputMsg::ShowError(format!(
                            "Error fetching releases: {}",
                            err
                        )))
                        .unwrap_or_else(|e| {
                            tracing::error!(error = ?e, "Error sending ShowError message");
                        });
                }
            },
            CommandMsg::ReleaseDeleted(result) => match result {
                Ok(deleted_id) => {
                    println!("Release deleted successfully with ID: {}", deleted_id);
                    sender.input(ReleasesMsg::FetchReleases);
                }
                Err(err) => {
                    tracing::error!(error = ?err, "Error deleting release");
                    sender
                        .output(ReleasesOutputMsg::ShowError(format!(
                            "Error deleting release: {}",
                            err
                        )))
                        .unwrap_or_else(|e| {
                            tracing::error!(
                                error = ?e,
                                "Error sending ShowError message");
                        });
                }
            },
        }
    }
}

impl ReleasesModel {
    fn get_selected_release_id(&self) -> Option<i64> {
        let selected_index = self.releases_list_view_wrapper.selection_model.selected();
        self.releases_list_view_wrapper
            .get_visible(selected_index)
            .map_or_else(|| None, |item| Some(item.borrow().id))
    }
}
