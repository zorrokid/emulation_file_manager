mod imp;

use glib::Object;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

glib::wrapper! {
    pub struct SimpleDialog(ObjectSubclass<imp::SimpleDialog>)
        @extends gtk::Dialog, gtk::Window, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SimpleDialog {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}
