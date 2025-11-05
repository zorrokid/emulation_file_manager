mod argument_list;
mod document_file_set_viewer;
mod document_viewer_form;
mod emulator_form;
mod emulator_runner;
mod file_importer;
mod file_info_details;
mod file_set_details_view;
mod file_set_editor;
mod file_set_form;
mod file_set_selector;
mod image_fileset_viewer;
mod image_viewer;
mod list_item;
mod logging;
mod release;
mod release_form;
mod releases;
mod settings_form;
mod software_title_form;
mod software_title_selector;
mod software_titles_list;
mod status_bar;
mod system_form;
mod system_selector;
mod tabbed_image_viewer;

use std::{path::PathBuf, sync::Arc};

use async_std::{channel::unbounded, task};
use cloud_storage::SyncEvent;
use database::{get_db_pool, repository_manager::RepositoryManager};
use release::{ReleaseInitModel, ReleaseModel, ReleaseMsg, ReleaseOutputMsg};
use releases::{ReleasesInit, ReleasesModel, ReleasesMsg, ReleasesOutputMsg};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    gtk::{
        self, FileChooserDialog,
        gio::{self, prelude::*},
        glib::clone,
        prelude::*,
    },
    once_cell::sync::OnceCell,
};
use service::{
    cloud_sync::service::CloudStorageSyncService,
    view_model_service::ViewModelService,
    view_models::{Settings, SoftwareTitleListModel},
};
use software_titles_list::{SoftwareTitleListInit, SoftwareTitlesList};

use crate::{
    settings_form::{SettingsForm, SettingsFormMsg},
    status_bar::{StatusBarModel, StatusBarMsg},
};

#[derive(Debug)]
struct InitResult {
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    settings: Arc<Settings>,
}

#[derive(Debug)]
enum AppMsg {
    Initialize,
    SoftwareTitleSelected { id: i64 },
    SoftwareTitleCreated(SoftwareTitleListModel),
    SoftwareTitleUpdated(SoftwareTitleListModel),
    ReleaseSelected { id: i64 },
    ExportAllFiles,
    ExportFolderSelected(PathBuf),
    SyncWithCloud,
    ProcessFileSyncEvent(SyncEvent),
    OpenSettings,
    UpdateSettings,
}

#[derive(Debug)]
enum CommandMsg {
    InitializationDone(InitResult),
    ExportFinished(Result<(), service::error::Error>),
}

struct AppModel {
    repository_manager: OnceCell<Arc<RepositoryManager>>,
    view_model_service: OnceCell<Arc<ViewModelService>>,
    settings: OnceCell<Arc<Settings>>,
    sync_service: OnceCell<Arc<CloudStorageSyncService>>,
    software_titles: OnceCell<Controller<SoftwareTitlesList>>,
    releases_view: gtk::Box,
    releases: OnceCell<Controller<ReleasesModel>>,
    release_view: gtk::Box,
    release: OnceCell<Controller<ReleaseModel>>,
    settings_form: OnceCell<Controller<settings_form::SettingsForm>>,
    status_bar: Controller<StatusBarModel>,
}

struct AppWidgets {
    sync_button: gtk::Button,
}

impl Component for AppModel {
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = CommandMsg;
    type Init = ();
    type Root = gtk::Window;
    type Widgets = AppWidgets;

    fn init_root() -> Self::Root {
        gtk::Window::builder()
            .title("EFCM")
            .default_width(800)
            .default_height(800)
            .build()
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let sync_button = Self::build_header_bar(&root, &sender);

        let main_container = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        let main_layout_hbox = gtk::Paned::builder()
            .orientation(gtk::Orientation::Horizontal)
            .vexpand(true)
            .build();

        let left_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .spacing(10)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .build();

        let right_vbox = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        main_layout_hbox.set_start_child(Some(&left_vbox));
        main_layout_hbox.set_end_child(Some(&right_vbox));

        let title_label = gtk::Label::builder().label("Software Titles").build();
        left_vbox.append(&title_label);

        main_container.append(&main_layout_hbox);
        let status_bar = StatusBarModel::builder().launch(()).detach();
        main_container.append(status_bar.widget());
        root.set_child(Some(&main_container));

        let widgets = AppWidgets { sync_button };

        let model = AppModel {
            repository_manager: OnceCell::new(),
            view_model_service: OnceCell::new(),
            settings: OnceCell::new(),
            releases_view: left_vbox, // both software titles and releases will be in left_vbox
            release_view: right_vbox,
            releases: OnceCell::new(),
            release: OnceCell::new(),
            software_titles: OnceCell::new(),
            sync_service: OnceCell::new(),
            settings_form: OnceCell::new(),
            status_bar,
        };

        sender.input(AppMsg::Initialize);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            AppMsg::Initialize => {
                sender.oneshot_command(async {
                    // TODO: Replace `expect` calls with proper error handling.
                    //       Instead of panicking on initialization failure,
                    //       return a `Result<InitResult, InitError>` and handle it in
                    //       `CommandMsg::InitializationDone`.
                    let pool = get_db_pool().await.expect("DB pool initialization failed");
                    let repository_manager = Arc::new(RepositoryManager::new(pool));
                    let view_model_service =
                        Arc::new(ViewModelService::new(Arc::clone(&repository_manager)));
                    let settings = view_model_service
                        .get_settings()
                        .await
                        .expect("Failed to get config");
                    let settings = Arc::new(settings);

                    CommandMsg::InitializationDone(InitResult {
                        repository_manager,
                        view_model_service,
                        settings,
                    })
                });
            }
            AppMsg::SoftwareTitleSelected { id } => {
                self.releases
                    .get()
                    .expect("ReleasesModel not initialized")
                    .emit(ReleasesMsg::SoftwareTitleSelected { id });
                self.release
                    .get()
                    .expect("Release widget not initialized")
                    .sender()
                    .emit(ReleaseMsg::Clear);
            }
            AppMsg::SoftwareTitleCreated(software_title_list_model) => {
                self.software_titles
                    .get()
                    .expect("SoftwareTitlesList not initialized")
                    .emit(
                        software_titles_list::SoftwareTitleListMsg::AddSoftwareTitle(
                            software_title_list_model.clone(),
                        ),
                    );
            }
            AppMsg::SoftwareTitleUpdated(_software_title_list_model) => {
                // TODO: update software title in list
            }
            AppMsg::ReleaseSelected { id } => {
                self.release
                    .get()
                    .expect("ReleasesModel not initialized")
                    .emit(ReleaseMsg::ReleaseSelected { id });
            }
            AppMsg::ExportAllFiles => {
                println!("Export All Files requested!");
                let dialog = FileChooserDialog::builder()
                    .title("Select folder to export all files")
                    .action(gtk::FileChooserAction::SelectFolder)
                    .modal(true)
                    .transient_for(root)
                    .build();

                dialog.add_button("Cancel", gtk::ResponseType::Cancel);
                dialog.add_button("Open", gtk::ResponseType::Accept);

                dialog.connect_response(clone!(
                    #[strong]
                    sender,
                    move |dialog, response| {
                        if response == gtk::ResponseType::Accept
                            && let Some(path) = dialog.file().and_then(|f| f.path())
                        {
                            sender.input(AppMsg::ExportFolderSelected(path));
                        }
                        dialog.close();
                    }
                ));

                dialog.present();
            }
            AppMsg::ExportFolderSelected(path) => {
                if path.is_dir() {
                    let repository_manager = Arc::clone(
                        self.repository_manager
                            .get()
                            .expect("Repository manager not initialized"),
                    );
                    let view_model_service = Arc::clone(
                        self.view_model_service
                            .get()
                            .expect("View model service not initialized"),
                    );
                    let settings =
                        Arc::clone(self.settings.get().expect("Settings not initialized"));
                    sender.oneshot_command(async move {
                        let export_service = service::export_service::ExportService::new(
                            repository_manager,
                            view_model_service,
                            settings,
                        );
                        let res = export_service.export_all_files(&path).await;
                        CommandMsg::ExportFinished(res)
                    });
                } else {
                    eprintln!("Selected path is not a directory: {:?}", path);
                }
            }
            AppMsg::SyncWithCloud => {
                let sync_service = self
                    .sync_service
                    .get()
                    .expect("Sync service not initialized");
                let sync_service_clone = Arc::clone(sync_service);
                let ui_sender = sender.clone();

                task::spawn(async move {
                    let (tx, rx) = unbounded::<SyncEvent>();

                    // Spawn task to forward progress messages to UI
                    task::spawn(async move {
                        let ui_sender = ui_sender.clone();
                        while let Ok(event) = rx.recv().await {
                            ui_sender.input(AppMsg::ProcessFileSyncEvent(event));
                        }
                    });

                    if let Err(e) = sync_service_clone.sync_to_cloud(tx).await {
                        eprintln!("Error during sync: {}", e);
                    }
                });
            }
            AppMsg::ProcessFileSyncEvent(event) => match event {
                SyncEvent::SyncStarted { total_files_count } => {
                    self.status_bar.emit(StatusBarMsg::StartProgress {
                        total: total_files_count,
                    });
                }
                SyncEvent::FileUploadStarted { .. } => {}
                SyncEvent::PartUploaded { .. } => {}
                SyncEvent::FileUploadCompleted {
                    file_number,
                    total_files,
                    ..
                } => {
                    self.status_bar.emit(StatusBarMsg::UpdateProgress {
                        done: file_number,
                        total: total_files,
                    });
                }
                SyncEvent::FileUploadFailed { .. } => {
                    // self.status_bar.emit(StatusBarMsg::Fail(error));
                }
                SyncEvent::SyncCompleted { .. } => {
                    self.status_bar.emit(StatusBarMsg::Finish);
                }
                SyncEvent::PartUploadFailed { error, .. } => {
                    // self.status_bar.emit(StatusBarMsg::Fail(error));
                }
                _ => { /* Handle other events as needed */ }
            },
            AppMsg::OpenSettings => {
                if self.settings_form.get().is_none() {
                    let settings_form_init = settings_form::SettingsFormInit {
                        repository_manager: Arc::clone(
                            self.repository_manager
                                .get()
                                .expect("Repository manager not initialized"),
                        ),
                        settings: Arc::clone(
                            self.settings.get().expect("Settings not initialized"),
                        ),
                    };
                    let settings_form = SettingsForm::builder()
                        .transient_for(root)
                        .launch(settings_form_init)
                        .forward(sender.input_sender(), |msg| match msg {
                            settings_form::SettingsFormOutputMsg::SettingsChanged => {
                                AppMsg::UpdateSettings
                            }
                        });
                    self.settings_form
                        .set(settings_form)
                        .expect("SettingsForm already initialized");
                }
                self.settings_form
                    .get()
                    .expect("SettingsForm not initialized")
                    .emit(SettingsFormMsg::Show);
            }
            AppMsg::UpdateSettings => {
                // TODO
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            CommandMsg::InitializationDone(init_result) => {
                let view_model_service = Arc::clone(&init_result.view_model_service);

                let software_title_list_init = SoftwareTitleListInit { view_model_service };

                let software_titles_list = SoftwareTitlesList::builder()
                    .launch(software_title_list_init)
                    .forward(sender.input_sender(), |msg| match msg {
                        software_titles_list::SoftwareTitleListOutMsg::SoftwareTitleSelected {
                            id,
                        } => AppMsg::SoftwareTitleSelected { id },
                    });

                let view_model_service = Arc::clone(&init_result.view_model_service);
                let repository_manager = Arc::clone(&init_result.repository_manager);
                let releases_init = ReleasesInit {
                    view_model_service,
                    repository_manager,
                    settings: Arc::clone(&init_result.settings),
                };

                let releases = ReleasesModel::builder().launch(releases_init).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        ReleasesOutputMsg::SoftwareTitleCreated {
                            software_title_list_model,
                        } => AppMsg::SoftwareTitleCreated(software_title_list_model),
                        ReleasesOutputMsg::SoftwareTitleUpdated {
                            software_title_list_model,
                        } => AppMsg::SoftwareTitleUpdated(software_title_list_model),
                        ReleasesOutputMsg::ReleaseSelected { id } => AppMsg::ReleaseSelected { id },
                    },
                );

                self.releases_view.append(software_titles_list.widget());
                self.software_titles
                    .set(software_titles_list)
                    .expect("software_titles already set");
                self.releases_view.append(releases.widget());
                self.releases.set(releases).expect("releases already set");

                let release_init_model = ReleaseInitModel {
                    view_model_service: Arc::clone(&init_result.view_model_service),
                    repository_manager: Arc::clone(&init_result.repository_manager),
                    settings: Arc::clone(&init_result.settings),
                };
                let release_model = ReleaseModel::builder().launch(release_init_model).forward(
                    sender.input_sender(),
                    |msg| match msg {
                        ReleaseOutputMsg::SoftwareTitleCreated(software_title_list_model) => {
                            AppMsg::SoftwareTitleCreated(software_title_list_model)
                        }
                    },
                );
                self.release_view.append(release_model.widget());

                let sync_service = Arc::new(CloudStorageSyncService::new(
                    Arc::clone(&init_result.repository_manager),
                    Arc::clone(&init_result.settings),
                ));

                self.view_model_service
                    .set(init_result.view_model_service)
                    .expect("view model service already initialized?");
                self.repository_manager
                    .set(init_result.repository_manager)
                    .expect("repository manger already initialized");
                self.settings
                    .set(init_result.settings)
                    .expect("Settings already initialized");
                self.sync_service
                    .set(sync_service)
                    .expect("Sync service already initialized");

                self.release
                    .set(release_model)
                    .expect("ReleaseModel already initialized");
            }
            CommandMsg::ExportFinished(result) => match result {
                Ok(_) => {
                    // TODO: show success dialog
                    println!("Export completed successfully.");
                }
                Err(e) => {
                    // TODO: show error dialog
                    eprintln!("Export failed: {}", e);
                }
            },
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let is_sync_enabled = self
            .settings
            .get()
            .map(|s| s.s3_sync_enabled)
            .unwrap_or(false);
        widgets.sync_button.set_sensitive(is_sync_enabled);
    }
}

impl AppModel {
    fn build_header_bar(root: &gtk::Window, sender: &ComponentSender<Self>) -> gtk::Button {
        let header_bar = gtk::HeaderBar::new();
        let export_button = gtk::Button::builder()
            .icon_name("document-save-symbolic")
            .tooltip_text("Export All Files")
            .build();

        export_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(AppMsg::ExportAllFiles);
            }
        ));

        header_bar.pack_end(&export_button);

        let sync_button = gtk::Button::builder()
            .icon_name("folder-sync-symbolic")
            .tooltip_text("Sync with Cloud Storage")
            .build();

        sync_button.connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                sender.input(AppMsg::SyncWithCloud);
            }
        ));

        sync_button.set_sensitive(false);

        header_bar.pack_end(&sync_button);

        let menu_button = gtk::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .tooltip_text("Menu")
            .build();

        let menu = gio::Menu::new();
        menu.append(Some("Settings"), Some("app.settings"));
        let popover = gtk::PopoverMenu::from_model(Some(&menu));

        menu_button.set_popover(Some(&popover));

        header_bar.pack_start(&menu_button);

        let settings_action = gio::SimpleAction::new("settings", None);
        settings_action.connect_activate(clone!(
            #[strong]
            sender,
            move |_, _| {
                sender.input(AppMsg::OpenSettings);
                println!("Settings action activated");
            }
        ));
        let app = relm4::main_application();
        app.add_action(&settings_action);

        root.set_titlebar(Some(&header_bar));
        sync_button
    }
}

fn main() {
    // Initialize logging - keep guard alive for entire program
    let _logging_guard = logging::init_logging();
    
    tracing::info!("Starting EFM Relm4 UI");
    
    let app = RelmApp::new("org.zorrokid.efcm");
    app.run::<AppModel>(());
    
    tracing::info!("Application shutdown");
}
