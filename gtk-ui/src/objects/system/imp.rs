use std::cell::RefCell;

use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use super::SystemData;

#[derive(Properties, Default)]
#[properties(wrapper_type = super::SystemObject)]
pub struct SystemObject {
    #[property(name = "id", get, set, type = i64, member = id)]
    #[property(name = "name", get, set, type = String, member = name)]
    pub data: RefCell<SystemData>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SystemObject {
    const NAME: &'static str = "EmuFilesSystemObject";
    type Type = super::SystemObject;
}

#[glib::derived_properties]
impl ObjectImpl for SystemObject {}
