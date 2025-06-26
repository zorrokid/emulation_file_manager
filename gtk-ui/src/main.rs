use gtk::prelude::*;
use gtk::{glib, Application};

const APP_ID: &str = "org.zorrokid.emufiles";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("Emu Files")
        .build();

    window.present();
}
