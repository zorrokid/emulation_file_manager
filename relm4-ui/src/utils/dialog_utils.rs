use std::{cell::RefCell, path::PathBuf, rc::Rc};

use relm4::gtk::{
    self,
    gio::prelude::FileExt,
    prelude::{DialogExt, FileChooserExt, GtkWindowExt, WidgetExt},
};

/// Show a message dialog with the specified message and type
/// # Arguments
/// * `message` - The message to display
/// * `message_type` - The type of message (Info, Warning, Error, Question)
/// * `root` - The root window to attach the dialog to
pub fn show_message_dialog(message: String, message_type: gtk::MessageType, root: &gtk::Window) {
    let dialog = gtk::MessageDialog::new(
        Some(root),
        gtk::DialogFlags::MODAL,
        message_type,
        gtk::ButtonsType::Ok,
        &message,
    );
    dialog.connect_response(|dialog, _| {
        dialog.close();
    });
    dialog.show();
}

/// Show an error dialog with the specified message
/// # Arguments
/// * `message` - The error message to display
/// * `root` - The root window to attach the dialog to
pub fn show_error_dialog(message: String, root: &gtk::Window) {
    show_message_dialog(message, gtk::MessageType::Error, root);
}

/// Show an info dialog with the specified message
/// # Arguments
/// * `message` - The info message to display
/// * `root` - The root window to attach the dialog to
pub fn show_info_dialog(message: String, root: &gtk::Window) {
    show_message_dialog(message, gtk::MessageType::Info, root);
}

pub fn show_file_chooser_dialog<F>(
    root: &gtk::Window,
    title: &str,
    action: gtk::FileChooserAction,
    on_accept: F,
) where
    // FnOnce: We want to call the closure only once, and it might need to
    // take ownership of its data. FnOnce is for closures that can be called just one time.
    F: FnOnce(PathBuf) + 'static,
{
    let dialog = gtk::FileChooserDialog::builder()
        .title(title)
        .action(action)
        .modal(true)
        .transient_for(root)
        .build();
    dialog.add_button("Cancel", gtk::ResponseType::Cancel);
    dialog.add_button("Select", gtk::ResponseType::Accept);

    // RefCell lets us change (mutate) something even if we only have a
    // shared reference to it. We need this because we want to remove (take)
    // the closure out of its Option when we use it.
    let on_accept_ref = RefCell::new(Some(on_accept));

    // Rc (Reference Counted) allows us to share ownership of the closure
    // between this function and the closure inside connect_response, which
    // may be called after this function returns.
    let on_accept = Rc::new(on_accept_ref);

    // Clone the Rc so the closure inside connect_response can access the same data.
    let accept_clone = Rc::clone(&on_accept);

    dialog.connect_response(move |dialog, response| {
        if response == gtk::ResponseType::Accept
            && let Some(path) = dialog.file().and_then(|f| f.path())

            // try to take the closure out of the RefCell (so it can only be called once)
            // We use take() so closure is removed from the RefCell and cannot be called again.
            // which enforces the single-use (FnOnce) nature of the closure.
            && let Some(on_accept) = accept_clone.borrow_mut().take()
        {
            // Call the closure with the selected file path.
            on_accept(path);
        }
        dialog.close();
    });

    dialog.present();
}
