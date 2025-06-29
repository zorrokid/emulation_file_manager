mod components;
mod objects;
mod util;
mod window;

use async_std::task;
use objects::repository_manager::RepositoryManagerObject;
use objects::view_model_service::ViewModelServiceObject;
use service::view_model_service::ViewModelService;
use std::sync::Arc;

use database::get_db_pool;
use database::repository_manager::RepositoryManager;
use gtk::prelude::*;
use gtk::{gio, glib, Application};
use window::Window;

const APP_ID: &str = "org.zorrokid.emufiles";

fn main() {
    task::block_on(async_main());
}

async fn async_main() {
    // Async DB pool setup
    let pool = match get_db_pool().await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Failed connecting to database: {}", err);
            return;
        }
    };
    let repo_manager = Arc::new(RepositoryManager::new(pool));
    let view_model_service = Arc::new(ViewModelService::new(Arc::clone(&repo_manager)));
    // Register and include resources
    gio::resources_register_include!("emufiles.gresource").expect("Failed to register resources.");

    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(move |app| {
        build_ui(app, repo_manager.clone(), view_model_service.clone());
    });
    app.run();
}

fn build_ui(
    app: &Application,
    repo_manager: Arc<RepositoryManager>,
    view_model_service: Arc<ViewModelService>,
) {
    // Create a new custom window and present it

    let repo_manager = RepositoryManagerObject::new(repo_manager);
    let view_model_service = ViewModelServiceObject::new(view_model_service);
    let window = Window::new(app, repo_manager, view_model_service);
    window.present();
}
