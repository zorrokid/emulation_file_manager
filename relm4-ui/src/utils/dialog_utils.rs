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

pub fn show_error_dialog(message: String, root: &gtk::Window) {
    show_message_dialog(message, gtk::MessageType::Error, root);
}

pub fn show_info_dialog(message: String, root: &gtk::Window) {
    show_message_dialog(message, gtk::MessageType::Info, root);
}
