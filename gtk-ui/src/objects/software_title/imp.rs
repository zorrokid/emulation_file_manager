use std::cell::RefCell;

use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use super::SoftwareTitleData;

// Object holding the state
#[derive(Properties, Default)]
#[properties(wrapper_type = super::SoftwareTitleObject)]
pub struct SoftwareTitleObject {
    #[property(name = "name", get, set, type = String, member = name)]
    pub data: RefCell<SoftwareTitleData>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for SoftwareTitleObject {
    const NAME: &'static str = "EmuFilesSoftwareTitleObject";
    type Type = super::SoftwareTitleObject;
}

// Trait shared by all GObjects
#[glib::derived_properties]
impl ObjectImpl for SoftwareTitleObject {}
