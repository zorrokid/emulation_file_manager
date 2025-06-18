pub mod custom_button;
use custom_button::CustomButton;
use gtk::prelude::*;
use gtk::{glib, Application};

const APP_ID: &str = "org.zorrokid.efm";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let button_increase = CustomButton::new();
    button_increase.set_margin_top(50);
    button_increase.set_margin_bottom(50);
    button_increase.set_margin_start(50);
    button_increase.set_margin_end(50);

    let gtk_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    // gtk_box keeps strong references to its children (keeps those buttons alive),
    gtk_box.append(&button_increase);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("SCM")
        .child(&gtk_box)
        .build();

    window.present();
}
