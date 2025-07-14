use std::sync::Arc;

use database::{get_db_pool, repository_manager::RepositoryManager};
use gtk::prelude::*;
use relm4::{
    RelmApp, RelmWidgetExt,
    component::{AsyncComponent, AsyncComponentParts, AsyncComponentSender},
    gtk,
    loading_widgets::LoadingWidgets,
    view,
};
use service::{view_model_service::ViewModelService, view_models::SoftwareTitleListModel};

struct App {
    counter: u8,
    repository_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
    software_titles: Vec<SoftwareTitleListModel>,
}

#[derive(Debug)]
enum Msg {
    Increment,
    Decrement,
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = u8;
    type Input = Msg;
    type Output = ();
    type CommandOutput = ();

    view! {
        gtk::Window {
            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 5,
                set_margin_all: 5,

                gtk::Button {
                    set_label: "Increment",
                    connect_clicked => Msg::Increment,
                },

                gtk::Button {
                    set_label: "Decrement",
                    connect_clicked => Msg::Decrement,
                },

                gtk::Label {
                    #[watch]
                    set_label: &format!("Counter: {}", model.counter),
                    set_margin_all: 5,
                }
            }
        }
    }

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Simple app"),
                set_default_size: (300, 100),

                // This will be removed automatically by
                // LoadingWidgets when the full view has loaded
                #[name(spinner)]
                gtk::Spinner {
                    start: (),
                    set_halign: gtk::Align::Center,
                }
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    async fn init(
        counter: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let pool = get_db_pool().await.expect("DB pool initialization failed");
        let repository_manager = Arc::new(RepositoryManager::new(pool));
        let view_model_service = Arc::new(ViewModelService::new(Arc::clone(&repository_manager)));
        let software_titles = view_model_service
            .get_software_title_list_models()
            .await
            .expect("Fetching software titles failed");

        let model = App {
            counter,
            repository_manager,
            view_model_service,
            software_titles,
        };

        // Insert the code generation of the view! macro here
        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            Msg::Increment => {
                self.counter = self.counter.wrapping_add(1);
                let res = self
                    .repository_manager
                    .get_system_repository()
                    .add_system(&"Test".to_string())
                    .await;

                match res {
                    Ok(id) => println!("Added system with id {}", id),
                    Err(err) => println!("Error while adding system: {}", err),
                }
            }
            Msg::Decrement => {
                self.counter = self.counter.wrapping_sub(1);
                let res = self.view_model_service.get_system_list_models().await;
                match res {
                    Ok(systems) => {
                        for system in systems {
                            println!("Got system: {}", system);
                        }
                    }
                    Err(err) => {
                        println!("Failed fetching systems: {}", err);
                    }
                }
            }
        }
    }
}

fn main() {
    let app = RelmApp::new("relm4.example.simple_async");
    app.run_async::<App>(0);
}
