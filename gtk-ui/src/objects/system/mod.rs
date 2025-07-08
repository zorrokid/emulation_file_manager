mod imp;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct SystemObject(ObjectSubclass<imp::SystemObject>);
}

impl SystemObject {
    pub fn new(id: i64, name: String) -> Self {
        Object::builder()
            .property("id", id)
            .property("name", name)
            .build()
    }
}

#[derive(Default)]
pub struct SystemData {
    pub id: i64,
    pub name: String,
}
