mod imp;

use gtk::glib;

glib::wrapper! {
    pub struct ReleaseFormWindow(ObjectSubclass<imp::ReleaseFormWindow>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ReleaseFormWindow {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
