use std::sync::Arc;

use database::{database_error::Error, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, glib,
        prelude::{
            BoxExt, ButtonExt, EditableExt, EntryBufferExtManual, EntryExt, GtkWindowExt,
            OrientableExt, WidgetExt,
        },
    },
    once_cell::sync::OnceCell,
    typed_view::list::TypedListView,
};
use service::{
    view_model_service::ViewModelService,
    view_models::{
        FileSetListModel, ReleaseViewModel, Settings, SoftwareTitleListModel, SystemListModel,
    },
};

use crate::{
    file_set_editor::FileSetEditor,
    file_set_selector::{
        FileSetSelector, FileSetSelectorInit, FileSetSelectorMsg, FileSetSelectorOutputMsg,
    },
    list_item::ListItem,
    software_title_selector::{
        SoftwareTitleSelectInit, SoftwareTitleSelectModel, SoftwareTitleSelectMsg,
        SoftwareTitleSelectOutputMsg,
    },
    system_selector::{
        SystemSelectInit, SystemSelectModel, SystemSelectMsg, SystemSelectOutputMsg,
    },
    utils::dialog_utils::{show_error_dialog, show_info_dialog},
};

#[derive(Debug)]
pub enum ReleaseFormMsg {
    OpenSystemSelector,
    OpenFileSelector,
    SystemSelected(SystemListModel),
    FileSetSelected(FileSetListModel),
    SoftwareTitleSelected(SoftwareTitleListModel),
    StartSaveRelease,
    OpenSoftwareTitleSelector,
    SoftwareTitleCreated(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
    UnlinkSoftwareTitle,
    UnlinkSystem,
    UnlinkFileSet,
    Show { release: Option<ReleaseViewModel> },
    Hide,
    EditFileSet,
    FileSetUpdated(FileSetListModel),
    NameChanged(String),
}

#[derive(Debug)]
pub enum ReleaseFormOutputMsg {
    ReleaseCreatedOrUpdated { id: i64 },
    SoftwareTitleCreated(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
}

#[derive(Debug)]
pub enum CommandMsg {
    ReleaseCreatedOrUpdated(Result<i64, Error>),
}

#[derive(Debug)]
pub struct ReleaseFormModel {
    view_model_service: Arc<ViewModelService>,
    repository_manager: Arc<RepositoryManager>,
    settings: Arc<Settings>,
    system_selector: Controller<SystemSelectModel>,
    file_selector: Controller<FileSetSelector>,
    software_title_selector: Controller<SoftwareTitleSelectModel>,
    selected_software_titles_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    selected_file_sets_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    release: Option<ReleaseViewModel>,
    file_set_editor: OnceCell<Controller<FileSetEditor>>,
    release_name: String,
}

pub struct ReleaseFormInit {
    pub view_model_service: Arc<ViewModelService>,
    pub repository_manager: Arc<RepositoryManager>,
    pub settings: Arc<Settings>,
    //pub release: Option<ReleaseViewModel>,
}

impl ReleaseFormModel {
    fn ensure_file_set_editor(&mut self, root: &gtk::Window, sender: &ComponentSender<Self>) {
        if self.file_set_editor.get().is_none() {
            let file_set_editor_init = crate::file_set_editor::FileSetEditorInit {
                view_model_service: Arc::clone(&self.view_model_service),
                repository_manager: Arc::clone(&self.repository_manager),
            };
            let file_set_editor = FileSetEditor::builder()
                .transient_for(root)
                .launch(file_set_editor_init)
                .forward(sender.input_sender(), |msg| match msg {
                    crate::file_set_editor::FileSetEditorOutputMsg::FileSetUpdated(file_set) => {
                        ReleaseFormMsg::FileSetUpdated(file_set)
                    }
                });
            if let Err(e) = self.file_set_editor.set(file_set_editor) {
                tracing::error!("Failed to set file set editor: {:?}", e);
            }
        }
    }
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

            connect_close_request[sender] => move |_| {
                sender.input(ReleaseFormMsg::Hide);
                glib::Propagation::Proceed
            },


            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,

                #[name="release_name_entry"]
                gtk::Entry {
                    set_text: &model.release_name,
                    set_placeholder_text: Some("Release name"),
                    connect_changed[sender] => move |entry| {
                        let buffer = entry.buffer();
                        sender.input(ReleaseFormMsg::NameChanged(buffer.text().into()));
                    },
                },


                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "Software titles",
                    },
                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[local_ref]
                        selected_software_titles_list_view -> gtk::ListView {}
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,

                        gtk::Button {
                            set_label: "Select Software Title",
                            connect_clicked => ReleaseFormMsg::OpenSoftwareTitleSelector,
                        },
                        gtk::Button {
                            set_label: "Unlink Software Title",
                            connect_clicked => ReleaseFormMsg::UnlinkSoftwareTitle,
                        },
                    },
                },


                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "Systems",
                    },

                    gtk::ScrolledWindow {
                        set_vexpand: true,
                        #[local_ref]
                        selected_systems_list_view -> gtk::ListView {}
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,

                        gtk::Button {
                            set_label: "Select System",
                            connect_clicked => ReleaseFormMsg::OpenSystemSelector,
                        },
                        gtk::Button {
                            set_label: "Unlink System",
                            connect_clicked => ReleaseFormMsg::UnlinkSystem,
                        },
                    },

                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Label {
                        set_label: "File sets",
                    },

                   gtk::ScrolledWindow {
                        set_min_content_height: 360,
                        set_vexpand: true,

                        #[local_ref]
                        selected_file_sets_list_view -> gtk::ListView {}

                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                         gtk::Button {
                            set_label: "Select File Set",
                            connect_clicked => ReleaseFormMsg::OpenFileSelector,
                        },
                        gtk::Button {
                            set_label: "Edit File Set",
                            connect_clicked => ReleaseFormMsg::EditFileSet,
                        },
                        gtk::Button {
                            set_label: "Unlink File Set",
                            connect_clicked => ReleaseFormMsg::UnlinkFileSet,
                        },
                    },
                },


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
        let selected_systems_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let selected_file_sets_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::new();

        let selected_software_titles_list_view_wrapper: TypedListView<
            ListItem,
            gtk::SingleSelection,
        > = TypedListView::new();

        let software_title_selector = SoftwareTitleSelectModel::builder()
            .transient_for(&root)
            .launch(SoftwareTitleSelectInit {
                view_model_service: Arc::clone(&init_model.view_model_service),
                repository_manager: Arc::clone(&init_model.repository_manager),
            })
            .forward(sender.input_sender(), |msg| match msg {
                SoftwareTitleSelectOutputMsg::Selected(software_title) => {
                    ReleaseFormMsg::SoftwareTitleSelected(software_title)
                }
                SoftwareTitleSelectOutputMsg::Created(software_title) => {
                    ReleaseFormMsg::SoftwareTitleCreated(software_title)
                }
                SoftwareTitleSelectOutputMsg::Updated(software_title) => {
                    ReleaseFormMsg::SoftwareTitleUpdated(software_title)
                }
            });

        let system_selector_init_model = SystemSelectInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
            repository_manager: Arc::clone(&init_model.repository_manager),
        };

        let system_selector = SystemSelectModel::builder()
            .transient_for(&root)
            .launch(system_selector_init_model)
            .forward(sender.input_sender(), |msg| match msg {
                SystemSelectOutputMsg::SystemSelected(system_list_model) => {
                    ReleaseFormMsg::SystemSelected(system_list_model)
                }
            });

        let file_selector_init_model = FileSetSelectorInit {
            view_model_service: Arc::clone(&init_model.view_model_service),
            repository_manager: Arc::clone(&init_model.repository_manager),
            settings: Arc::clone(&init_model.settings),
        };
        let file_selector = FileSetSelector::builder()
            .transient_for(&root)
            .launch(file_selector_init_model)
            .forward(sender.input_sender(), |msg| match msg {
                FileSetSelectorOutputMsg::FileSetSelected(file_set_liset_model) => {
                    ReleaseFormMsg::FileSetSelected(file_set_liset_model)
                }
            });

        let model = ReleaseFormModel {
            view_model_service: init_model.view_model_service,
            repository_manager: init_model.repository_manager,
            settings: init_model.settings,
            release: None, // init_model.release,
            system_selector,
            file_selector,
            software_title_selector,
            selected_software_titles_list_view_wrapper,
            selected_systems_list_view_wrapper,
            selected_file_sets_list_view_wrapper,
            file_set_editor: OnceCell::new(),
            release_name: String::new(),
        };

        let selected_systems_list_view = &model.selected_systems_list_view_wrapper.view;
        let selected_file_sets_list_view = &model.selected_file_sets_list_view_wrapper.view;
        let selected_software_titles_list_view =
            &model.selected_software_titles_list_view_wrapper.view;
        let widgets = view_output!();
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
            ReleaseFormMsg::OpenSystemSelector => {
                self.system_selector.emit(SystemSelectMsg::Show {
                    selected_system_ids: get_item_ids(&self.selected_systems_list_view_wrapper),
                });
            }
            ReleaseFormMsg::OpenFileSelector => {
                self.file_selector.emit(FileSetSelectorMsg::Show {
                    selected_system_ids: get_item_ids(&self.selected_systems_list_view_wrapper),
                    selected_file_set_ids: get_item_ids(&self.selected_file_sets_list_view_wrapper),
                });
            }
            ReleaseFormMsg::OpenSoftwareTitleSelector => {
                self.software_title_selector
                    .emit(SoftwareTitleSelectMsg::Show {
                        selected_software_title_ids: get_item_ids(
                            &self.selected_software_titles_list_view_wrapper,
                        ),
                    });
            }

            ReleaseFormMsg::SystemSelected(system) => {
                self.selected_systems_list_view_wrapper.append(ListItem {
                    name: system.name.clone(),
                    id: system.id,
                });
            }
            ReleaseFormMsg::FileSetSelected(file_set) => {
                self.selected_file_sets_list_view_wrapper.append(ListItem {
                    name: file_set.file_set_name.clone(),
                    id: file_set.id,
                });
            }
            ReleaseFormMsg::SoftwareTitleSelected(software_title) => {
                self.selected_software_titles_list_view_wrapper
                    .append(ListItem {
                        name: software_title.name.clone(),
                        id: software_title.id,
                    });
            }
            ReleaseFormMsg::StartSaveRelease => {
                tracing::info!("Starting to save release with selected systems and file sets");
                let repository_manager = Arc::clone(&self.repository_manager);
                let software_title_ids =
                    get_item_ids(&self.selected_software_titles_list_view_wrapper);
                let system_ids = get_item_ids(&self.selected_systems_list_view_wrapper);

                let file_set_ids = get_item_ids(&self.selected_file_sets_list_view_wrapper);

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
                                tracing::info!(
                                    "Editing existing release {} with id: {}",
                                    release_name,
                                    id
                                );
                                repository_manager
                                    .get_release_repository()
                                    .update_release_full(
                                        id,
                                        release_name.as_str(),
                                        &software_title_ids,
                                        &file_set_ids,
                                        &system_ids,
                                    )
                                    .await
                            }
                            _ => {
                                tracing::info!("Creating new release with name: {}", release_name);
                                repository_manager
                                    .get_release_repository()
                                    .add_release_full(
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
                tracing::info!("Software title created: {:?}", &software_title);
                let res = sender.output(ReleaseFormOutputMsg::SoftwareTitleCreated(software_title));
                if let Err(msg) = res {
                    tracing::error!("Error in sending message {:?}", msg);
                }
            }
            ReleaseFormMsg::SoftwareTitleUpdated(software_title) => {
                tracing::info!("Software title updated: {:?}", &software_title);
                let res = sender.output(ReleaseFormOutputMsg::SoftwareTitleUpdated(software_title));
                if let Err(msg) = res {
                    tracing::error!("Error in sending message {:?}", msg);
                }
            }
            ReleaseFormMsg::UnlinkSoftwareTitle => {
                remove_selected(&mut self.selected_software_titles_list_view_wrapper);
            }
            ReleaseFormMsg::UnlinkSystem => {
                remove_selected(&mut self.selected_systems_list_view_wrapper);
            }
            ReleaseFormMsg::UnlinkFileSet => {
                remove_selected(&mut self.selected_file_sets_list_view_wrapper);
            }
            ReleaseFormMsg::Show { release } => {
                let mut selected_systems = vec![];
                let mut selected_file_sets = vec![];
                let mut selected_software_titles = vec![];
                if let Some(release) = release {
                    tracing::info!("Showing release form with release: {:?}", &release);
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
                    let release_name = release.name.clone();
                    widgets.release_name_entry.set_text(release_name.as_str());
                    self.release_name = release.name.clone();
                    self.release = Some(release);
                } else {
                    tracing::info!("Showing release form for new release");
                    widgets.release_name_entry.set_text("");
                    self.release_name = String::new();
                    self.release = None;
                }

                self.selected_systems_list_view_wrapper.clear();
                self.selected_systems_list_view_wrapper.extend_from_iter(
                    selected_systems.iter().map(|s| ListItem {
                        id: s.id,
                        name: s.name.clone(),
                    }),
                );
                self.selected_file_sets_list_view_wrapper.clear();
                self.selected_file_sets_list_view_wrapper.extend_from_iter(
                    selected_file_sets.iter().map(|fs| ListItem {
                        id: fs.id,
                        name: fs.file_set_name.clone(),
                    }),
                );
                self.selected_software_titles_list_view_wrapper.clear();
                self.selected_software_titles_list_view_wrapper
                    .extend_from_iter(selected_software_titles.iter().map(|st| ListItem {
                        id: st.id,
                        name: st.name.clone(),
                    }));
                root.show();
            }
            ReleaseFormMsg::Hide => {
                root.hide();
            }
            ReleaseFormMsg::EditFileSet => {
                let selected = self
                    .selected_file_sets_list_view_wrapper
                    .selection_model
                    .selected();
                if let Some(file_set) = self
                    .selected_file_sets_list_view_wrapper
                    .get_visible(selected)
                {
                    let file_set_id = file_set.borrow().id;
                    self.ensure_file_set_editor(root, &sender);
                    self.file_set_editor
                        .get()
                        .expect("File set editor should be initialized")
                        .emit(crate::file_set_editor::FileSetEditorMsg::Show { file_set_id });
                }
            }
            ReleaseFormMsg::FileSetUpdated(file_set) => {
                for i in 0..self.selected_file_sets_list_view_wrapper.len() {
                    if let Some(item) = self.selected_file_sets_list_view_wrapper.get(i)
                        && item.borrow().id == file_set.id
                    {
                        item.borrow_mut().name = file_set.file_set_name.clone();
                        break;
                    }
                }
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
                tracing::info!("Release created or updated with ID: {}", id);
                let res = sender.output(ReleaseFormOutputMsg::ReleaseCreatedOrUpdated { id });
                if let Err(e) = res {
                    tracing::error!("Error sending output message: {:?}", e);
                } else {
                    root.close();
                }
            }
            CommandMsg::ReleaseCreatedOrUpdated(Err(err)) => {
                show_error_dialog(
                    format!("Failed to create or update release: {:?}", err),
                    root,
                );
            }
        }
    }
}

fn get_item_ids(list_view_wrapper: &TypedListView<ListItem, gtk::SingleSelection>) -> Vec<i64> {
    (0..list_view_wrapper.len())
        .filter_map(|i| list_view_wrapper.get(i).map(|st| st.borrow().id))
        .collect()
}

fn remove_selected(list_view_wrapper: &mut TypedListView<ListItem, gtk::SingleSelection>) {
    list_view_wrapper.remove(list_view_wrapper.selection_model.selected());
}
