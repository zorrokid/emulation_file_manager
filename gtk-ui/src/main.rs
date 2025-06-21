pub mod custom_button;
use custom_button::CustomButton;
use gtk::glib::closure_local;
use gtk::prelude::*;
use gtk::{glib, Application};

const APP_ID: &str = "org.zorrokid.efm";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &Application) {
    let button_1 = CustomButton::new();
    button_1.set_margin_top(50);
    button_1.set_margin_bottom(50);
    button_1.set_margin_start(50);
    button_1.set_margin_end(50);
    let button_2 = CustomButton::new();

    button_1
        .bind_property("number", &button_2, "number")
        .transform_to(|_, number: i32| {
            let incremented_number = number + 1;
            Some(incremented_number)
        })
        .transform_to(|_, number: i32| {
            let decremented_number = number - 1;
            Some(decremented_number)
        })
        .bidirectional()
        .sync_create()
        .build();

    button_1.connect_number_notify(|button| {
        let number = button.number();
        println!("Button 1 number changed: {}", number);
    });

    button_1.connect_closure(
        "max-number-reached",
        false,
        closure_local!(move |_button: &CustomButton, number: i32| {
            println!("Max number reached on button 1: {}", number);
        }),
    );

    let gtk_box = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();
    // gtk_box keeps strong references to its children (keeps those buttons alive),
    gtk_box.append(&button_1);
    gtk_box.append(&button_2);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("SCM")
        .child(&gtk_box)
        .build();

    window.present();
}
