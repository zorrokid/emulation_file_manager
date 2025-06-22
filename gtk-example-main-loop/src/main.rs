use std::thread;
use std::time::Duration;

use gtk::glib::clone;
use gtk::{self, Application, ApplicationWindow, Box, Button, glib};
use gtk::{gio, prelude::*};

const APP_ID: &str = "org.gtk_rs.MainEventLoop1";

fn main() -> glib::ExitCode {
    // Create a new application
    let app = Application::builder().application_id(APP_ID).build();

    // Connect to "activate" signal of `app`
    app.connect_activate(build_ui);

    // Run the application
    app.run()
}

fn build_ui(app: &Application) {
    // Button with sync blocking action
    let button_sync = Button::builder()
        .label("Press me (sync)!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Create channel that can hold at most 1 message at a time
    let (sender, receiver) = async_channel::bounded(1);
    // Connect to "clicked" signal of `button`
    button_sync.connect_clicked(move |_| {
        let sender = sender.clone();
        // The long running operation runs now in a separate thread
        gio::spawn_blocking(move || {
            // Deactivate the button until the operation is done
            sender
                .send_blocking(false)
                .expect("The channel needs to be open.");
            let five_seconds = Duration::from_secs(5);
            thread::sleep(five_seconds);
            // Activate the button again
            sender
                .send_blocking(true)
                .expect("The channel needs to be open.");
        });
    });

    // The main loop executes the asynchronous block
    glib::spawn_future_local(clone!(
        #[weak]
        button_sync,
        async move {
            while let Ok(enable_button) = receiver.recv().await {
                button_sync.set_sensitive(enable_button);
            }
        }
    ));

    // Button with async action
    let button_async = Button::builder()
        .label("Press me (async)!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    button_async.connect_clicked(move |button| {
        glib::spawn_future_local(clone!(
            #[weak]
            button,
            async move {
                button.set_sensitive(false);
                glib::timeout_future_seconds(5).await;
                button.set_sensitive(true);
            }
        ));
    });

    let gtk_box = Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .build();

    gtk_box.append(&button_sync);
    gtk_box.append(&button_async);

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .child(&gtk_box)
        .build();

    // Present window
    window.present();
}
