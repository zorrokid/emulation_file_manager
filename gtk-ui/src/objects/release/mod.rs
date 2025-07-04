mod imp;

use glib::Object;
use gtk::glib;

glib::wrapper! {
    pub struct ReleaseObject(ObjectSubclass<imp::ReleaseObject>);
}

impl ReleaseObject {
    pub fn new(id: i64, name: String, system_names: Vec<String>, file_types: Vec<String>) -> Self {
        Object::builder()
            .property("id", id)
            .property("name", name)
            .property("system_names", system_names)
            .property("file_types", file_types)
            .build()
    }
}

#[derive(Default)]
pub struct ReleaseData {
    pub id: i64,
    pub name: String,
    pub system_names: Vec<String>,
    pub file_types: Vec<String>,
}
