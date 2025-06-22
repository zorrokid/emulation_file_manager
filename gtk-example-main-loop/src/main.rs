use std::thread;
use std::time::Duration;

use gtk::glib::clone;
use gtk::{self, Application, ApplicationWindow, Button, glib};
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
    // Create a button
    let button = Button::builder()
        .label("Press me!")
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();

    // Create channel that can hold at most 1 message at a time
    let (sender, receiver) = async_channel::bounded(1);

    // Connect to "clicked" signal of `button`
    button.connect_clicked(move |_| {
        let sender = sender.clone();
        // The long running operation runs now in a separate thread
        gio::spawn_blocking(move || {
            // Deactivate the button until the operation is done
            sender
                .send_blocking(false)
                .expect("Failed to send message to channel");
            let five_seconds = Duration::from_secs(5);
            thread::sleep(five_seconds);
            // Reactivate the button after the operation is done
            sender
                .send_blocking(true)
                .expect("Failed to send message to channel");
        });
    });

    // The main loop executes the asynchronous block
    glib::spawn_future_local(clone!(
        #[weak]
        button,
        async move {
            while let Ok(enable_button) = receiver.recv().await {
                button.set_sensitive(enable_button);
            }
        }
    ));

    // Create a window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .child(&button)
        .build();

    // Present window
    window.present();
}
