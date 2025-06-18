use gtk::glib;
use gtk::subclass::prelude::*;

// object holding the state
#[derive(Default)]
pub struct CustomButton;

// central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for CustomButton {
    // NAME should consist of crate-name and object-name in order to avoid name collisions. Use UpperCamelCase here.
    const NAME: &'static str = "GtkUiCustomButton";
    // Type refers to the actual GObject that will be created afterwards.
    type Type = super::CustomButton;
    // ParentType is the GObject we inherit of.
    type ParentType = gtk::Button;
}

// trait shared by all GObjects
impl ObjectImpl for CustomButton {}

// trait shared by all widgets
impl WidgetImpl for CustomButton {}

// trait shared by all buttons
impl ButtonImpl for CustomButton {}
