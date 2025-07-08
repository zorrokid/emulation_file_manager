mod imp;

use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct SystemDialog(ObjectSubclass<imp::SystemDialog>)
        @extends gtk::Dialog, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SystemDialog {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn get_system_name(&self) -> Option<String> {
        let entry = self.imp().system_name_entry.get();
        let text = entry.text();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}
