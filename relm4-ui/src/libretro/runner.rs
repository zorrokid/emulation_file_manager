use std::sync::Arc;

use domain::models::System;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    gtk::{
        self, Window,
        glib::{self, clone},
        prelude::{ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use service::{
    app_services::AppServices,
    error::Error,
    libretro::core::service::{CoreMappingModel, LibretroCoreInfo},
    libretro::error::LibretroPreflightError,
    libretro::runner::service::{LibretroLaunchModel, LibretroLaunchPaths},
    view_models::{FileSetFileInfoViewModel, FileSetViewModel},
};
use ui_components::string_list_view::{
    StringListView, StringListViewInit, StringListViewMsg, StringListViewOutput,
};

use crate::{
    libretro::{LibretroWindowModel, LibretroWindowMsg, LibretroWindowOutput},
    list_item::ListItem,
    utils::dialog_utils::show_error_dialog,
};

#[derive(Debug)]
pub enum LibretroRunnerMsg {
    FetchCores {
        system_id: i64,
    },
    // list selection messages
    FileSelected {
        index: u32,
    },
    CoreSelected {
        name: Option<String>,
    },
    SystemSelected {
        index: u32,
    },
    Show {
        file_set: FileSetViewModel,
        systems: Vec<System>,
    },
    Hide,
    StartCore,
    ShowError(String),
    LibretroSessionEnded(Vec<String>),
}

#[derive(Debug)]
pub enum LibretroRunnerCommandMsg {
    CoresFetched { cores: Vec<String> },

    FinishedRunningCore(Result<(), Error>),
    ProcessCoresResult(Result<Vec<CoreMappingModel>, Error>),
    FilesPrepared(Result<LibretroLaunchPaths, LibretroPreflightError>),
    ProcessSystemInfoResult(Result<LibretroCoreInfo, LibretroPreflightError>),
}

pub struct LibretroRunnerInit {
    pub app_services: Arc<AppServices>,
}

#[derive(Debug)]
pub struct LibretroRunner {
    libretro_window: Controller<LibretroWindowModel>,

    // services
    app_services: Arc<AppServices>,

    // list views
    file_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
    core_list_view_wrapper: Controller<StringListView<String>>,
    system_list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,

    // data
    cores: Vec<String>,
    systems: Vec<System>,
    core_info: Option<LibretroCoreInfo>,

    // needed for running the core:
    file_set: Option<FileSetViewModel>,
    selected_file: Option<FileSetFileInfoViewModel>,
    selected_system: Option<System>,
    selected_core: Option<String>,
}

impl LibretroRunner {
    pub fn can_launch_core(&self) -> bool {
        // Let's not do firmware checks here for now, since the UI doesn't currently indicate
        // firmware requirements/availability. Now if firmware is missing, an error message will
        // just be shown when trying to launch the core.
        self.selected_core.is_some() && self.selected_file.is_some() && self.file_set.is_some()
    }
}

#[relm4::component(pub)]
impl Component for LibretroRunner {
    type Init = LibretroRunnerInit;
    type Input = LibretroRunnerMsg;
    type Output = ();
    type CommandOutput = LibretroRunnerCommandMsg;

    view! {
        #[root]
        gtk::Window {
            connect_close_request[sender] => move |_| {
                sender.input(LibretroRunnerMsg::Hide);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                file_list_view -> gtk::ListView,

                #[local_ref]
                system_list_view -> gtk::ListView,

                model.core_list_view_wrapper.widget(),

                gtk::Button {
                    set_label: "Start",
                    connect_clicked => LibretroRunnerMsg::StartCore,
                    #[watch]
                    set_sensitive: model.can_launch_core(),
                },

            },
        }
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let file_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();
        let system_list_view_wrapper = TypedListView::<ListItem, gtk::SingleSelection>::new();

        let core_list_view_wrapper = StringListView::builder()
            .launch(StringListViewInit {
                title: "Available Cores".to_string(),
            })
            .forward(sender.input_sender(), |msg| match msg {
                StringListViewOutput::SelectionChanged(name) => {
                    LibretroRunnerMsg::CoreSelected { name }
                }
            });

        let libretro_window = LibretroWindowModel::builder().launch(()).forward(
            sender.input_sender(),
            |msg| match msg {
                LibretroWindowOutput::Error(e) => LibretroRunnerMsg::ShowError(e),
                LibretroWindowOutput::SessionEnded(files) => {
                    LibretroRunnerMsg::LibretroSessionEnded(files)
                }
            },
        );

        let model = LibretroRunner {
            app_services: init.app_services,
            file_list_view_wrapper,
            core_list_view_wrapper,
            system_list_view_wrapper,
            cores: Vec::new(),
            systems: Vec::new(),
            file_set: None,
            selected_file: None,
            selected_system: None,
            selected_core: None,
            libretro_window,
            core_info: None,
        };

        let file_list_view = &model.file_list_view_wrapper.view;
        let system_list_view = &model.system_list_view_wrapper.view;

        model
            .file_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(LibretroRunnerMsg::FileSelected { index: selected });
                }
            ));

        model
            .system_list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |selection| {
                    let selected = selection.selected();
                    sender.input(LibretroRunnerMsg::SystemSelected { index: selected });
                }
            ));

        let widgets = view_output!();
        sender.input(LibretroRunnerMsg::SystemSelected { index: 0 });
        sender.input(LibretroRunnerMsg::FileSelected { index: 0 });
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            LibretroRunnerMsg::FileSelected { index } => {
                self.handle_file_selection(index);
            }
            LibretroRunnerMsg::SystemSelected { index } => {
                self.handle_system_selection(index, &sender);
            }
            LibretroRunnerMsg::CoreSelected { name } => {
                // TODO: once core is selected, core info should be loaded and availability of
                // possible firmware should be indicated in the UI.
                // Also core info should be passed to the libretro window (it will be used for
                // example to set the InputProfile.
                self.handle_core_selection(name, &sender);
            }
            LibretroRunnerMsg::StartCore => {
                self.handle_start_core(&sender, root);
            }
            LibretroRunnerMsg::Show { file_set, systems } => {
                self.init_with_new_data(file_set, systems, &sender);
                root.show();
            }
            LibretroRunnerMsg::Hide => {
                root.hide();
            }
            LibretroRunnerMsg::FetchCores { system_id } => {
                let app_services = Arc::clone(&self.app_services);
                sender.oneshot_command(async move {
                    let cores_res = app_services
                        .libretro_core()
                        .get_cores_for_system(system_id)
                        .await;
                    LibretroRunnerCommandMsg::ProcessCoresResult(cores_res)
                });
            }
            LibretroRunnerMsg::ShowError(e) => {
                show_error_dialog(e, root);
            }
            LibretroRunnerMsg::LibretroSessionEnded(files) => {
                self.app_services.libretro_runner().cleanup_files(&files);
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
            LibretroRunnerCommandMsg::CoresFetched { cores } => {
                self.cores = cores;
                self.core_list_view_wrapper
                    .emit(StringListViewMsg::SetItems(self.cores.clone()));
            }
            LibretroRunnerCommandMsg::FinishedRunningCore(result) => {
                if let Err(e) = result {
                    eprintln!("Error running core: {:?}", e);
                }
            }
            LibretroRunnerCommandMsg::ProcessCoresResult(result) => match result {
                Ok(core_mappings) => {
                    self.cores = core_mappings.into_iter().map(|m| m.core_name).collect();
                    self.core_list_view_wrapper
                        .emit(StringListViewMsg::SetItems(self.cores.clone()));
                }
                Err(e) => {
                    eprintln!("Error fetching cores for system: {:?}", e);
                }
            },
            LibretroRunnerCommandMsg::FilesPrepared(Ok(paths)) => {
                if let Some(core_info) = &self.core_info {
                    self.libretro_window.emit(LibretroWindowMsg::Launch {
                        core_path: paths.core_path,
                        rom_path: paths.rom_path,
                        system_dir: paths.system_dir,
                        temp_files: paths.temp_files,
                        input_profile: core_info.input_profile,
                    });
                }
            }
            LibretroRunnerCommandMsg::FilesPrepared(Err(e)) => {
                tracing::error!(error = ?e, "Failed to prepare files for core launch");
                show_error_dialog(e.to_string(), root);
            }
            LibretroRunnerCommandMsg::ProcessSystemInfoResult(Ok(result)) => {
                self.core_info = Some(result);
            }
            LibretroRunnerCommandMsg::ProcessSystemInfoResult(Err(e)) => {
                tracing::error!(error = ?e, "Failed to fetch system info for core");
                show_error_dialog(e.to_string(), root);
            }
        }
    }
}

impl LibretroRunner {
    pub fn handle_file_selection(&mut self, index: u32) {
        let file_list_item = self.file_list_view_wrapper.get(index);
        if let (Some(item), Some(file_set)) = (file_list_item, &self.file_set) {
            let id = item.borrow().id;
            let file_info = file_set.files.iter().find(|f| f.file_info_id == id);
            self.selected_file = file_info.cloned();
        }
    }

    pub fn handle_system_selection(&mut self, index: u32, sender: &ComponentSender<Self>) {
        let system_list_item = self.system_list_view_wrapper.get(index);
        if let Some(item) = system_list_item {
            let id = item.borrow().id;
            let system = self.systems.iter().find(|s| s.id == id);
            self.selected_system = system.cloned();
            if let Some(system) = system {
                sender.input(LibretroRunnerMsg::FetchCores {
                    system_id: system.id,
                });
            }
        }
    }

    pub fn handle_core_selection(&mut self, name: Option<String>, sender: &ComponentSender<Self>) {
        self.selected_core = name;
        if let Some(name) = &self.selected_core {
            let core_name = name.clone();
            let service = Arc::clone(&self.app_services);
            sender.oneshot_command(async move {
                LibretroRunnerCommandMsg::ProcessSystemInfoResult(
                    service
                        .libretro_core()
                        .get_core_system_info(core_name.as_str())
                        .await,
                )
            });
        }
    }

    pub fn handle_start_core(&mut self, sender: &ComponentSender<Self>, root: &Window) {
        if let (Some(core_name), Some(file_set), Some(file_info), Some(core_info)) = (
            self.selected_core.clone(),
            self.file_set.clone(),
            self.selected_file.clone(),
            &self.core_info,
        ) {
            // first need to prepare the files
            let app_services = Arc::clone(&self.app_services);
            let core_info = core_info.clone();
            match app_services.libretro_runner().resolve_core_path(&core_name) {
                Ok(core_path) => {
                    sender.oneshot_command(async move {
                        LibretroRunnerCommandMsg::FilesPrepared(
                            app_services
                                .libretro_runner()
                                .prepare_rom(LibretroLaunchModel {
                                    file_set_id: file_set.id,
                                    initial_file: Some(file_info.file_name.clone()),
                                    core_path,
                                    core_info,
                                })
                                .await,
                        )
                    });
                }
                Err(e) => {
                    tracing::error!(error = ?e, "Failed to resolve core path");
                    show_error_dialog(e.to_string(), root);
                }
            }
        }
    }

    pub fn init_with_new_data(
        &mut self,
        file_set: FileSetViewModel,
        systems: Vec<System>,
        sender: &ComponentSender<Self>,
    ) {
        let file_list_items = file_set
            .files
            .iter()
            .map(|file| ListItem {
                id: file.file_info_id,
                name: file.file_name.clone(),
            })
            .collect::<Vec<_>>();

        self.file_list_view_wrapper.clear();
        self.file_list_view_wrapper
            .extend_from_iter(file_list_items);

        sender.input(LibretroRunnerMsg::FileSelected {
            index: self.file_list_view_wrapper.selection_model.selected(),
        });

        let system_list_items = systems
            .iter()
            .map(|system| ListItem {
                id: system.id,
                name: system.name.clone(),
            })
            .collect::<Vec<_>>();

        self.system_list_view_wrapper.clear();
        self.system_list_view_wrapper
            .extend_from_iter(system_list_items);

        sender.input(LibretroRunnerMsg::FetchCores {
            system_id: systems.first().map_or(0, |s| s.id),
        });

        self.systems = systems;
        self.file_set = Some(file_set);
    }
}
