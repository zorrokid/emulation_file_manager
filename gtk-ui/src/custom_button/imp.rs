use std::cell::Cell;
use std::sync::OnceLock;

use glib::types::StaticType;
use gtk::glib::object::ObjectExt;
use gtk::glib::subclass::Signal;
use gtk::glib::{self, Properties};
use gtk::prelude::ButtonExt;
use gtk::subclass::prelude::*;

// object holding the state
#[derive(Properties, Default)]
#[properties(wrapper_type = super::CustomButton)]
pub struct CustomButton {
    #[property(get, set)]
    number: Cell<i32>,
}

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

#[glib::derived_properties]
// trait shared by all GObjects
impl ObjectImpl for CustomButton {
    // This function is called when the object is constructed.
    fn constructed(&self) {
        self.parent_constructed();
        self.obj().set_label(&self.number.get().to_string());

        // Bind the "label" property to the number property
        let obj = self.obj();
        obj.bind_property("number", obj.as_ref(), "label")
            .sync_create()
            .build();
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
        SIGNALS.get_or_init(|| {
            vec![Signal::builder("max-number-reached")
                .param_types([i32::static_type()])
                .build()]
        })
    }
}

// trait shared by all widgets
impl WidgetImpl for CustomButton {}

static MAX_NUMBER: i32 = 6;

// trait shared by all buttons
impl ButtonImpl for CustomButton {
    fn clicked(&self) {
        let incremented_number = self.obj().number() + 1;
        let obj = self.obj();
        if incremented_number > MAX_NUMBER {
            // Emit the custom signal if the max number is reached
            obj.emit_by_name::<()>("max-number-reached", &[&incremented_number]);
            obj.set_number(0);
        } else {
            obj.set_number(incremented_number);
        }
    }
}
