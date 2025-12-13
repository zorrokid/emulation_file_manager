use std::{collections::HashSet, sync::Arc};

use relm4::{
    ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    gtk::{
        self,
        glib::clone,
        prelude::{BoxExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    error::Error,
    view_model_service::{ReleaseFilter, ViewModelService},
    view_models::{FileSetViewModel, ReleaseListModel, SystemListModel},
};

use crate::{
    file_info_details::{
        FileInfoDetails, FileInfoDetailsInit, FileInfoDetailsMsg, FileInfoDetailsOutputMsg,
    },
    list_item::ListItem,
};

#[derive(Debug)]
pub struct FileSetDetailsView {
    view_model_service: Arc<ViewModelService>,
    files_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    systems_list_view_wrapper: TypedListView<ListItem, gtk::NoSelection>,
    software_titles_list_view_wrapper: TypedListView<ListItem, gtk::NoSelection>,
    file_info_details: Controller<FileInfoDetails>,
}

#[derive(Debug)]
pub enum FileSetDetailsMsg {
    LoadFileSet(i64),
    FileSelected { index: u32 },
    ShowError(String),
}

#[derive(Debug)]
pub enum FileSetDetailsCmdMsg {
    FileSetLoaded(Result<FileSetViewModel, Error>),
    ReleasesLoaded(Result<Vec<ReleaseListModel>, Error>),
    FileSetSystemsLoaded(Result<Vec<SystemListModel>, Error>),
}

#[derive(Debug)]
pub enum FileSetDetailsOutputMsg {
    ShowError(String),
}

#[derive(Debug)]
pub struct FileSetDetailsInit {
    pub view_model_service: Arc<ViewModelService>,
}

#[relm4::component(pub)]
impl relm4::Component for FileSetDetailsView {
    type Init = FileSetDetailsInit;
    type Input = FileSetDetailsMsg;
    type Output = FileSetDetailsOutputMsg;
    type CommandOutput = FileSetDetailsCmdMsg;

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 10,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 10,

                #[name = "file_set_name_label"]
                gtk::Label {
                    set_label: "File Set Details",
                },

                gtk::Label {
                    set_label: "Files in file set:",
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    files_list -> gtk::ListView {}
                },

                gtk::Label {
                    set_label: "Systems linked to file set:",
                },

               gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    systems_list -> gtk::ListView {}
                },

                gtk::Label {
                    set_label: "Software titles linked to file set:",
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    software_titles_list -> gtk::ListView {}
                }
            },
            #[local_ref]
            file_info_details_widget -> gtk::Box,
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let files_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();
        let systems_list_view_wrapper = TypedListView::<ListItem, gtk::NoSelection>::new();
        let software_titles_list_view_wrapper = TypedListView::<ListItem, gtk::NoSelection>::new();
        let file_info_details_init = FileInfoDetailsInit {
            view_model_service: Arc::clone(&init.view_model_service),
        };
        let file_info_details = FileInfoDetails::builder()
            .launch(file_info_details_init)
            .forward(sender.input_sender(), |msg| match msg {
                FileInfoDetailsOutputMsg::ShowError(error_msg) => {
                    FileSetDetailsMsg::ShowError(error_msg)
                }
            });

        let model = FileSetDetailsView {
            view_model_service: init.view_model_service,
            files_list_view_wrapper,
            systems_list_view_wrapper,
            software_titles_list_view_wrapper,
            file_info_details,
        };

        let files_list = &model.files_list_view_wrapper.view;
        let systems_list = &model.systems_list_view_wrapper.view;
        let software_titles_list = &model.software_titles_list_view_wrapper.view;
        model
            .files_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(FileSetDetailsMsg::FileSelected { index: selected });
                }
            ));

        let file_info_details_widget = model.file_info_details.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            FileSetDetailsMsg::LoadFileSet(file_set_id) => {
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let result = view_model_service
                        .get_file_set_view_model(file_set_id)
                        .await;
                    FileSetDetailsCmdMsg::FileSetLoaded(result)
                });

                let view_model_service = Arc::clone(&self.view_model_service.clone());
                sender.oneshot_command(async move {
                    let result = view_model_service
                        .get_release_list_models(ReleaseFilter {
                            file_set_id: Some(file_set_id),
                            ..Default::default()
                        })
                        .await;
                    FileSetDetailsCmdMsg::ReleasesLoaded(result)
                });
                let view_model_service = Arc::clone(&self.view_model_service);
                sender.oneshot_command(async move {
                    let result = view_model_service
                        .get_systems_for_file_set(file_set_id)
                        .await;
                    FileSetDetailsCmdMsg::FileSetSystemsLoaded(result)
                });
            }
            FileSetDetailsMsg::FileSelected { index } => {
                let selected_item = self.files_list_view_wrapper.get_visible(index);
                if let Some(item) = selected_item {
                    println!("Selected file: {:?}", item);
                    let id = item.borrow().id;
                    self.file_info_details
                        .emit(FileInfoDetailsMsg::LoadFileInfo(id));
                }
            }
            FileSetDetailsMsg::ShowError(msg) => {
                sender
                    .output(FileSetDetailsOutputMsg::ShowError(msg))
                    .unwrap_or_else(|err| {
                        tracing::error!(
                        error = ?err,
                        "Failed sending FileSetDetailsOutputMsg::ShowError")
                    });
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            FileSetDetailsCmdMsg::FileSetLoaded(Ok(file_set_view_model)) => {
                println!("Loaded File Set: {:?}", file_set_view_model);
                let items = file_set_view_model.files.into_iter().map(|file| ListItem {
                    id: file.file_info_id,
                    name: file.file_name.clone(),
                });
                self.files_list_view_wrapper.clear();
                self.files_list_view_wrapper.extend_from_iter(items);
            }
            FileSetDetailsCmdMsg::FileSetLoaded(Err(err)) => {
                eprintln!("Error loading File Set: {:?}", err);
            }
            FileSetDetailsCmdMsg::ReleasesLoaded(Ok(releases)) => {
                println!("Loaded Releases: {:?}", releases);
                let software_titles: HashSet<ListItem> = releases
                    .iter()
                    .flat_map(|release| {
                        release
                            .software_title_names
                            .iter()
                            .map(|title_name| ListItem {
                                id: 0, // TODO: add id
                                name: title_name.clone(),
                            })
                    })
                    .collect();
                self.software_titles_list_view_wrapper.clear();
                self.software_titles_list_view_wrapper
                    .extend_from_iter(software_titles);
            }
            FileSetDetailsCmdMsg::ReleasesLoaded(Err(err)) => {
                eprintln!("Error loading Releases: {:?}", err);
            }
            FileSetDetailsCmdMsg::FileSetSystemsLoaded(Ok(systems)) => {
                println!("Loaded Systems: {:?}", systems);
                let items = systems.into_iter().map(|system| ListItem {
                    id: system.id,
                    name: system.name.clone(),
                });
                self.systems_list_view_wrapper.clear();
                self.systems_list_view_wrapper.extend_from_iter(items);
            }
            FileSetDetailsCmdMsg::FileSetSystemsLoaded(Err(err)) => {
                eprintln!("Error loading Systems: {:?}", err);
            }
        }
    }
}
