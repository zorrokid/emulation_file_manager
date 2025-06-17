use glib::clone;
use gtk::prelude::*;
use gtk::{glib, Application, Button};
use std::cell::Cell;
use std::rc::Rc;

const APP_ID: &str = "org.zorrokid.scm";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let button_increase = Button::builder()
        .label("Increase")
        .margin_top(50)
        .margin_bottom(50)
        .margin_start(50)
        .margin_end(50)
        .build();

    let button_decrease = Button::builder()
        .label("Decrease")
        .margin_top(50)
        .margin_bottom(50)
        .margin_start(50)
        .margin_end(50)
        .build();

    let number = Rc::new(Cell::new(0));

    button_increase.connect_clicked(clone!(
        #[weak]
        number,
        #[strong]
        button_decrease,
        move |_| {
            number.set(number.get() + 1);
            button_decrease.set_label(&format!("Decrease: {}", number.get()));
        }
    ));
    button_decrease.connect_clicked(clone!(
        #[strong]
        button_increase,
        move |_| {
            number.set(number.get() - 1);
            button_increase.set_label(&format!("Increase: {}", number.get()));
        }
    ));

    let gtk_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    gtk_box.append(&button_increase);
    gtk_box.append(&button_decrease);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("SCM")
        .child(&gtk_box)
        .build();

    window.present();
}
