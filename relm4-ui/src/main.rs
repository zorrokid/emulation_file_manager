use std::sync::Arc;

use database::{get_db_pool, repository_manager::RepositoryManager};
use relm4::{
    Component, ComponentParts, ComponentSender, RelmApp, RelmWidgetExt,
    gtk::{
        self,
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt},
    },
    once_cell::sync::OnceCell,
};
use service::{view_model_service::ViewModelService, view_models::SoftwareTitleListModel};

#[derive(Debug)]
struct InitResult {
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    software_titles: Vec<SoftwareTitleListModel>,
}

#[derive(Debug)]
enum AppMsg {
    Increment,
    Decrement,
    Initialize,
}

#[derive(Debug)]
enum CommandMsg {
    InitializationDone(InitResult),
}

struct AppModel {
    counter: u8,
    software_titles: Vec<SoftwareTitleListModel>,
    repository_manager: OnceCell<Arc<RepositoryManager>>,
    view_model_service: OnceCell<Arc<ViewModelService>>,
}

struct AppWidgets {
    label: gtk::Label,
}

#[relm4::component]
impl Component for AppModel {
    /// The type of the messages that this component can receive.
    type Input = AppMsg;
    /// The type of the messages that this component can send.
    type Output = ();
    type CommandOutput = CommandMsg;
    /// The type of data with which this component will be initialized.
    type Init = u8;

    view! {
        gtk::Window {
            set_title: Some("Simple app"),
            set_default_width: 300,
            set_default_height: 100,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,

                gtk::Button {
                    set_label: "Increment",
                    connect_clicked => AppMsg::Increment
                },

                gtk::Button::with_label("Decrement") {
                    connect_clicked => AppMsg::Decrement
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!("Counter: {}", model.counter),
                    set_margin_all: 5,
                }
            }
        }
    }

    fn init(
        counter: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = AppModel {
            counter,
            software_titles: vec![],
            repository_manager: OnceCell::new(),
            view_model_service: OnceCell::new(),
        };

        // macro code generation
        let widgets = view_output!();
        sender.input(AppMsg::Initialize);
        sender.input(AppMsg::Increment);
        sender.input(AppMsg::Increment);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _: &Self::Root) {
        match msg {
            AppMsg::Increment => {
                self.counter = self.counter.wrapping_add(1);
            }
            AppMsg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
            }
            AppMsg::Initialize => {
                sender.oneshot_command(async {
                    let pool = get_db_pool().await.expect("DB pool initialization failed");
                    let repository_manager = Arc::new(RepositoryManager::new(pool));
                    let view_model_service =
                        Arc::new(ViewModelService::new(Arc::clone(&repository_manager)));
                    let software_titles = view_model_service
                        .get_software_title_list_models()
                        .await
                        .expect("Fetching software titles failed");
                    CommandMsg::InitializationDone(InitResult {
                        repository_manager,
                        view_model_service,
                        software_titles,
                    })
                });
            }
        }
    }
    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _: &Self::Root,
    ) {
        match message {
            CommandMsg::InitializationDone(init_result) => {
                self.view_model_service
                    .set(init_result.view_model_service)
                    .expect("view model service already initialized?");
                self.repository_manager
                    .set(init_result.repository_manager)
                    .expect("repository manger already initialized");
                self.software_titles = init_result.software_titles;
                dbg!(
                    "Software titles initialized: {}",
                    self.software_titles.len()
                );
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("relm4.test.simple_manual");
    app.run::<AppModel>(0);
}
