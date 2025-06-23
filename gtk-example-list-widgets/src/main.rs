use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Label, ListBox, PolicyType, ScrolledWindow, glib};

const APP_ID: &str = "org.gtk_rs.ListWidgets1";

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    let list_box = ListBox::new();
    for number in 1..=100 {
        let label = Label::new(Some(&number.to_string()));
        list_box.append(&label);
    }

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .min_content_width(360)
        .child(&list_box)
        .build();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("List Widgets Example")
        .default_height(600)
        .default_width(360)
        .child(&scrolled_window)
        .build();

    window.present();
}
