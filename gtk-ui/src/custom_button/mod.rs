mod imp;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct CustomButton(ObjectSubclass<imp::CustomButton>)
        @extends gtk::Widget, gtk::Button,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl CustomButton {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn with_label(label: &str) -> Self {
        Object::builder().property("label", label).build()
    }
}

impl Default for CustomButton {
    fn default() -> Self {
        Self::new()
    }
}
