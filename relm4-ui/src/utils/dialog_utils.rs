use relm4::gtk::{
    self,
    prelude::{DialogExt, GtkWindowExt, WidgetExt},
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
