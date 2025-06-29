mod imp;

use glib::{clone, Object};
use gtk::glib::{MainContext, WeakRef};
use gtk::subclass::prelude::*;
use gtk::{
    gio, glib, Application, ButtonsType, MessageDialog, MessageType, NoSelection,
    SignalListItemFactory,
};
use gtk::{prelude::*, ListItem};

use crate::components::software_title_row::SoftwareTitleRow;
use crate::objects::repository_manager::RepositoryManagerObject;
use crate::objects::software_title::SoftwareTitleObject;
use crate::objects::view_model_service::{self, ViewModelServiceObject};
use crate::util::error::show_error_dialog;

// define custome Window widget
glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    pub fn new(
        app: &Application,
        repo_manager: RepositoryManagerObject,
        view_model_service: ViewModelServiceObject,
    ) -> Self {
        let window: Self = Object::builder()
            .property("application", app)
            .property("repo-manager", repo_manager)
            .property("view-model-service", view_model_service)
            .build();
        window
    }

    fn software_titles(&self) -> gio::ListStore {
        // Get state
        self.imp()
            .software_titles
            .borrow()
            .clone()
            .expect("Could not get current software titles.")
    }

    fn setup_software_titles(&self) {
        // Create new model
        // gio::ListStore only accepts GObjects, that's why we use `SoftwareTitleObject` which is a
        // subclass of GObject.
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
                window.new_software_title();
            }
        ));

        // Setup callback for clicking (and the releasing) the icon of the entry
        self.imp().entry.connect_icon_release(clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _| {
                window.new_software_title();
            }
        ));
    }

    fn new_software_title(&self) {
        // Get content from entry and clear it
        let buffer = self.imp().entry.buffer();
        let content = buffer.text().to_string();
        if content.is_empty() {
            return;
        }
        buffer.set_text("");

        // Add new software title to model
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

    fn setup_property_callbacks(&self) {
        self.connect_notify_local(Some("view-model-service"), |window, _| {
            if window.view_model_service().is_some() {
                window.fetch_software_titles();
            }
        });
    }

    fn fetch_software_titles(&self) {
        let view_model_service = self.view_model_service();
        let list_store = self.software_titles();
        let gtk_window_weak: WeakRef<gtk::Window> = self.upcast_ref::<gtk::Window>().downgrade();

        MainContext::default().spawn_local(clone!(
            #[weak]
            list_store,
            async move {
                if let Some(service_object) = view_model_service {
                    match service_object.get_software_title_list_models().await {
                        Ok(titles) => {
                            for title in titles {
                                list_store.append(&title);
                            }
                        }
                        Err(err) => {
                            if let Some(gtk_window) = gtk_window_weak.upgrade() {
                                show_error_dialog(
                                    &gtk_window,
                                    &format!("Failed to fetch software titles; {err}"),
                                );
                            }
                        }
                    }
                }
            }
        ));
    }
}
