use std::cell::Cell;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{glib, Application, Button};

const APP_ID: &str = "org.zorrokid.scm";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let button = Button::builder()
        .label("Click Me")
        .margin_top(50)
        .margin_bottom(50)
        .margin_start(50)
        .margin_end(50)
        .build();

    let number = Rc::new(Cell::new(0));

    let number_clone = Rc::clone(&number);
    button.connect_clicked(move |_| {
        number_clone.set(number_clone.get() + 1);
    });
    button.connect_clicked(move |_| {
        number.set(number.get() - 1);
    });

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("SCM")
        .child(&button)
        .build();

    window.present();
}
