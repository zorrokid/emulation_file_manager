mod imp;

use glib::{clone, Object};
use gtk::subclass::prelude::*;
use gtk::{gio, glib, Application, NoSelection, SignalListItemFactory};
use gtk::{prelude::*, ListItem};

use crate::components::software_title_row::SoftwareTitleRow;
use crate::objects::software_title::SoftwareTitleObject;

// In order to use the composite template, we create a custom widget.
glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(app: &Application) -> Self {
        // Create new window
        Object::builder().property("application", app).build()
    }

    fn software_titles(&self) -> gio::ListStore {
        // Get state
        self.imp()
            .software_titles
            .borrow()
            .clone()
            .expect("Could not get current software titles.")
    }

    fn setup_tasks(&self) {
        // Create new model
        let model = gio::ListStore::new::<SoftwareTitleObject>();

        // Get state and set model
        self.imp().software_titles.replace(Some(model));

        // Wrap model with selection and pass it to the list view
        let selection_model = NoSelection::new(Some(self.software_titles()));
        self.imp()
            .software_titles_list
            .set_model(Some(&selection_model));
    }

    fn setup_callbacks(&self) {
        // Setup callback for activation of the entry
        self.imp().entry.connect_activate(clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.new_task();
            }
        ));

        // Setup callback for clicking (and the releasing) the icon of the entry
        self.imp().entry.connect_icon_release(clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _| {
                window.new_task();
            }
        ));
    }

    fn new_task(&self) {
        // Get content from entry and clear it
        let buffer = self.imp().entry.buffer();
        let content = buffer.text().to_string();
        if content.is_empty() {
            return;
        }
        buffer.set_text("");

        // Add new task to model
        let task = SoftwareTitleObject::new(content);
        self.software_titles().append(&task);
    }

    fn setup_factory(&self) {
        // Create a new factory
        let factory = SignalListItemFactory::new();

        // Create an empty `SoftwareTitleRow` during setup
        factory.connect_setup(move |_, list_item| {
            // Create `SoftwareTitleRow`
            let task_row = SoftwareTitleRow::new();
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&task_row));
        });

        // Tell factory how to bind `SoftwareTitleRow` to a `SoftwareTitleObject`
        factory.connect_bind(move |_, list_item| {
            // Get `SoftwareTitleObject` from `ListItem`
            let task_object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<SoftwareTitleObject>()
                .expect("The item has to be an `SoftwareTitleObject`.");

            // Get `SoftwareTitleRow` from `ListItem`
            let task_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<SoftwareTitleRow>()
                .expect("The child has to be a `SoftwareTitleRow`.");

            task_row.bind(&task_object);
        });

        // Tell factory how to unbind `SoftwareTitleRow` from `SoftwareTitleObject`
        factory.connect_unbind(move |_, list_item| {
            // Get `SoftwareTitleRow` from `ListItem`
            let task_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<SoftwareTitleRow>()
                .expect("The child has to be a `SoftwareTitleRow`.");

            task_row.unbind();
        });

        // Set the factory of the list view
        self.imp().software_titles_list.set_factory(Some(&factory));
    }
}
