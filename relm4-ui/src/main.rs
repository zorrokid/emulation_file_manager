mod app;
mod argument_list;
mod components;
mod document_file_set_viewer;
mod document_viewer_form;
mod emulator_form;
mod emulator_runner;
mod file_info_details;
mod file_set_details_view;
mod file_set_form;
mod file_set_selector;
mod image_fileset_viewer;
mod image_viewer;
mod import_form;
mod list_item;
mod logging;
mod release;
mod release_form;
mod release_form_components;
mod releases;
mod settings_form;
mod software_title_form;
mod software_title_merge_dialog;
mod software_title_selector;
mod software_titles_list;
mod status_bar;
mod style;
mod system_form;
mod system_selector;
mod tabbed_image_viewer;
mod utils;

use crate::app::AppModel;
use relm4::RelmApp;

fn main() {
    // Initialize logging - keep guard alive for entire program
    let _logging_guard = logging::init_logging();

    tracing::info!("Starting EFM Relm4 UI");

    let app = RelmApp::new("org.zorrokid.efcm");
    app.run::<AppModel>(());

    tracing::info!("Application shutdown");
}
