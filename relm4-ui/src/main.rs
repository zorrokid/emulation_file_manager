mod argument_list;
mod document_file_set_viewer;
mod document_viewer_form;
mod emulator_form;
mod emulator_runner;
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
mod utils;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use async_std::{channel::unbounded, task};
use core_types::events::SyncEvent;
use database::{get_db_pool, repository_manager::RepositoryManager};
use release::{ReleaseInitModel, ReleaseModel, ReleaseMsg, ReleaseOutputMsg};
use releases::{ReleasesInit, ReleasesModel, ReleasesMsg, ReleasesOutputMsg};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmApp,
    gtk::{
        self, FileChooserDialog,
        gio::{self, prelude::*},
        glib::{Propagation, clone},
        prelude::*,
    },
    once_cell::sync::OnceCell,
};
use service::{
    cloud_sync::service::{CloudStorageSyncService, SyncResult},
    view_model_service::ViewModelService,
    view_models::{Settings, SoftwareTitleListModel},
};
use software_titles_list::{SoftwareTitleListInit, SoftwareTitlesList};

use crate::{
    settings_form::{SettingsForm, SettingsFormMsg},
    status_bar::{StatusBarModel, StatusBarMsg},
    utils::dialog_utils::{show_error_dialog, show_info_dialog},
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
    CloseRequested,
}

#[derive(Debug)]
enum CommandMsg {
    InitializationDone(InitResult),
    ExportFinished(Result<(), service::error::Error>),
    SyncToCloudCompleted(Result<SyncResult, service::error::Error>),
}

struct Flags {
    app_closing: bool,
    cloud_sync_in_progress: bool,
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
    // wrapping the flags in single Mutex to prevent possible race conditions
    flags: Arc<Mutex<Flags>>,
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
        let flags = Arc::new(Mutex::new(Flags {
            app_closing: false,
            cloud_sync_in_progress: false,
        }));

        // Handle close request by checking the shared flag, we need to do this
        // because of possible running async tasks that may prevent immediate closing.
        root.connect_close_request(clone!(
            #[strong]
            sender,
            #[strong]
            flags,
            move |_| {
                // Check if app is already closing
                let flags = &mut *flags.lock().unwrap();

                // Allow closing if (1) no sync in progress or (2) user already confirmed closing
                // (app_closing flag set)
                if flags.cloud_sync_in_progress && !flags.app_closing {
                    // Send message to handle close logic
                    sender.input(AppMsg::CloseRequested);
                    Propagation::Stop
                } else {
                    // Default case, allow close
                    Propagation::Proceed
                }
            }
        ));

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
            flags,
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
                let should_start_sync = {
                    let mut flags = self.flags.lock().unwrap();

                    if flags.app_closing {
                        tracing::warn!("Sync requested but app is closing, ignoring");
                        false
                    } else if flags.cloud_sync_in_progress {
                        tracing::warn!("Sync already in progress, ignoring new request");
                        false
                    } else {
                        flags.cloud_sync_in_progress = true;
                        true
                    }
                };

                if !should_start_sync {
                    return;
                }

                let sync_service = self
                    .sync_service
                    .get()
                    .expect("Sync service not initialized");
                let sync_service = Arc::clone(sync_service);
                let ui_sender = sender.clone();

                let (tx, rx) = unbounded::<SyncEvent>();

                // Spawn task to forward progress messages to UI
                task::spawn(async move {
                    while let Ok(event) = rx.recv().await {
                        ui_sender.input(AppMsg::ProcessFileSyncEvent(event));
                    }
                });

                sender.oneshot_command(async move {
                    let res = sync_service.sync_to_cloud(tx).await;
                    CommandMsg::SyncToCloudCompleted(res)
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
                SyncEvent::PartUploadFailed { .. } => {
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
            AppMsg::CloseRequested => {
                let sync_in_progress = {
                    let flags = self.flags.lock().unwrap();

                    if flags.app_closing {
                        // Already closing, shouldn't reach here but just in case
                        return;
                    }
                    flags.cloud_sync_in_progress
                };

                if sync_in_progress {
                    let dialog = gtk::MessageDialog::builder()
                        .transient_for(root)
                        .modal(true)
                        .message_type(gtk::MessageType::Warning)
                        .buttons(gtk::ButtonsType::OkCancel)
                        .text("Cloud sync in progress")
                        .secondary_text(
                            "A cloud sync operation is currently running. \
                             Closing now will cancel the sync after the current file finishes uploading.\n\n\
                             Do you want to cancel the sync and close the application?"
                        )
                        .build();

                    let flags_clone = Arc::clone(&self.flags);
                    dialog.connect_response(clone!(
                        #[strong]
                        root,
                        #[strong]
                        flags_clone,
                        move |dialog, response| {
                            if response == gtk::ResponseType::Ok {
                                // User confirmed - set flag and trigger close
                                // TODO: Send cancel signal to sync operation when implemented
                                let flags = &mut *flags_clone.lock().unwrap();
                                flags.app_closing = true;
                                dialog.close();
                                root.close(); // Trigger close again, now flag is set
                            } else {
                                dialog.close();
                            }
                        }
                    ));

                    dialog.present();
                } else {
                    // No sync in progress, safe to close
                    tracing::info!("Application closing normally");
                    let mut flags = self.flags.lock().unwrap();
                    flags.app_closing = true;
                    drop(flags); // Release lock before closing
                    root.close(); // Trigger close, flag is now set so it will proceed
                }
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
            CommandMsg::SyncToCloudCompleted(result) => {
                let mut flags = self.flags.lock().unwrap();
                flags.cloud_sync_in_progress = false;
                drop(flags); // Release lock before showing dialog
                match result {
                    Ok(sync_result) => {
                        let message = format!(
                            "Cloud sync completed.\nSuccessful uploads: {}\nFailed uploads: {}\nSuccessful deletions: {}\nFailed deletions: {}",
                            sync_result.successful_uploads,
                            sync_result.failed_uploads,
                            sync_result.successful_deletions,
                            sync_result.failed_deletions
                        );
                        show_info_dialog(message, root);
                    }
                    Err(e) => {
                        show_error_dialog(format!("Cloud sync failed: {}", e), root);
                    }
                }
            }
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
