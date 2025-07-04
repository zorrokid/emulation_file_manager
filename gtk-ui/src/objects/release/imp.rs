use std::cell::RefCell;

use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use super::ReleaseData;

// Object holding the state
#[derive(Properties, Default)]
#[properties(wrapper_type = super::ReleaseObject)]
pub struct ReleaseObject {
    #[property(name = "id", get, set, type = i64, member = id)]
    #[property(name = "name", get, set, type = String, member = name)]
    #[property(name = "system_names", get, set, type = String, member = system_names)]
    #[property(name = "file_types", get, set, type = String, member = file_types)]
    pub data: RefCell<ReleaseData>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for ReleaseObject {
    const NAME: &'static str = "EmuFilesReleaseObject";
    type Type = super::ReleaseObject;
}

// Trait shared by all GObjects
#[glib::derived_properties]
impl ObjectImpl for ReleaseObject {}
