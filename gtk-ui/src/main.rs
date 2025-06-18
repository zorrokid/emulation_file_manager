pub mod custom_button;
use custom_button::CustomButton;
use glib::clone;
use gtk::prelude::*;
use gtk::{glib, Application, Button};
use std::cell::Cell;
use std::rc::Rc;

const APP_ID: &str = "org.zorrokid.efm";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let button_increase = CustomButton::with_label("Increase");
    button_increase.set_margin_top(50);
    button_increase.set_margin_bottom(50);
    button_increase.set_margin_start(50);
    button_increase.set_margin_end(50);

    let button_decrease = Button::builder()
        .label("Decrease")
        .margin_top(50)
        .margin_bottom(50)
        .margin_start(50)
        .margin_end(50)
        .build();

    // Rc is a reference counted pointer, which allows multiple ownership of the same value.
    // Cell is a type that allows for interior mutability, meaning that it can be mutated even when
    // the value is behind an immutable reference.
    let number = Rc::new(Cell::new(0));

    // The clone! macro is used to capture references to the widgets and number for use inside the closures, ensuring proper memory management with #[weak] to avoid reference cycles.
    //
    // A strong reference keeps the referenced value from being deallocated. If this
    // chain leads to a circle, none of the values in this cycle ever get deallocated.
    //
    // A weak reference does not keep the value from being deallocated. If the value is
    // deallocated, the weak reference will return None when accessed.
    //
    // Every time the button is clicked, glib::clone tries to upgrade the weak reference.
    // If we now for example click on one button and the other button is not there anymore, the callback will be skipped.
    // Per default, it immediately returns from the closure with () as return value.
    // In case the closure expects a different return value @default-return can be specified.

    button_increase.connect_clicked(clone!(
        #[weak]
        number,
        #[weak]
        button_decrease,
        move |_| {
            number.set(number.get() + 1);
            button_decrease.set_label(&format!("Decrease: {}", number.get()));
        }
    ));
    button_decrease.connect_clicked(clone!(
        // No weak reference for number here, as we want to keep it alive for both buttons.
        #[weak]
        button_increase,
        move |_| {
            number.set(number.get() - 1);
            button_increase.set_label(&format!("Increase: {}", number.get()));
        }
    ));

    let gtk_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    // gtk_box keeps strong references to its children (keeps those buttons alive),
    gtk_box.append(&button_increase);
    gtk_box.append(&button_decrease);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("SCM")
        .child(&gtk_box)
        .build();

    window.present();
}
