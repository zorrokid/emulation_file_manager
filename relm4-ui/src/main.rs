mod argument_list;
mod document_file_set_viewer;
mod document_viewer_form;
mod emulator_form;
mod emulator_runner;
mod file_importer;
mod file_selector;
mod file_set_editor;
mod file_set_form;
mod image_fileset_viewer;
mod image_viewer;
mod list_item;
mod release;
mod release_form;
mod releases;
mod software_title_form;
mod software_title_selector;
mod software_titles_list;
mod system_form;
mod system_selector;
mod tabbed_image_viewer;

use std::{path::PathBuf, sync::Arc};

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
    view_model_service::ViewModelService,
    view_models::{Settings, SoftwareTitleListModel},
};
use software_titles_list::{SoftwareTitleListInit, SoftwareTitlesList};

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
    software_titles: OnceCell<Controller<SoftwareTitlesList>>,
    releases_view: gtk::Box,
    releases: OnceCell<Controller<ReleasesModel>>,
    release_view: gtk::Box,
    release: OnceCell<Controller<ReleaseModel>>,
}

struct AppWidgets {}

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
        // Create header bar with simple button
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
        root.set_titlebar(Some(&header_bar));

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

        root.set_child(Some(&main_layout_hbox));

        let widgets = AppWidgets {};

        let model = AppModel {
            repository_manager: OnceCell::new(),
            view_model_service: OnceCell::new(),
            settings: OnceCell::new(),
            releases_view: left_vbox, // both software titles and releases will be in left_vbox
            release_view: right_vbox,
            releases: OnceCell::new(),
            release: OnceCell::new(),
            software_titles: OnceCell::new(),
        };

        sender.input(AppMsg::Initialize);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            AppMsg::Initialize => {
                sender.oneshot_command(async {
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
                        if response == gtk::ResponseType::Accept {
                            if let Some(path) = dialog.file().and_then(|f| f.path()) {
                                sender.input(AppMsg::ExportFolderSelected(path));
                            }
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
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _: &Self::Root,
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

                self.view_model_service
                    .set(init_result.view_model_service)
                    .expect("view model service already initialized?");
                self.repository_manager
                    .set(init_result.repository_manager)
                    .expect("repository manger already initialized");
                self.settings
                    .set(init_result.settings)
                    .expect("Settings already initialized");

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
}

fn main() {
    let app = RelmApp::new("org.zorrokid.efcm");
    app.run::<AppModel>(());
}
