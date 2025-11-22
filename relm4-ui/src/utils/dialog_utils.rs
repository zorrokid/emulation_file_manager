use relm4::gtk::{
    self,
    prelude::{DialogExt, GtkWindowExt, WidgetExt},
};

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
