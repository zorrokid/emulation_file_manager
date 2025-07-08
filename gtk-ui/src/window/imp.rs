use std::cell::RefCell;

use glib::subclass::InitializingObject;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate, Entry, ListView};

use crate::components::simple_dialog;
use crate::components::software_title_details::SoftwareTitleDetails;
use crate::objects::repository_manager::RepositoryManagerObject;
use crate::objects::view_model_service::ViewModelServiceObject;
use gtk::prelude::*;

// Object holding the state
#[derive(CompositeTemplate, Default)]
#[template(resource = "/org/zorrokid/emufiles/window.ui")]
pub struct Window {
    #[template_child]
    pub entry: TemplateChild<Entry>,
    #[template_child]
    pub software_titles_list: TemplateChild<ListView>,
    pub software_titles: RefCell<Option<gio::ListStore>>,
    pub repo_manager: RefCell<Option<RepositoryManagerObject>>,
    pub view_model_service: RefCell<Option<ViewModelServiceObject>>,
    #[template_child]
    pub details_pane: TemplateChild<SoftwareTitleDetails>,
    #[template_child(id = "header_bar")]
    pub header_bar: TemplateChild<gtk::HeaderBar>,
    #[template_child(id = "app_menu_button")]
    pub app_menu_button: TemplateChild<gtk::MenuButton>,
}

// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for Window {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "EmuFilesWindow";
    // This is the type of the object that this subclass will create
    type Type = super::Window;
    // The parent type (GObject that we inherit of)
    type ParentType = gtk::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for Window {
    fn constructed(&self) {
        // Call "constructed" on parent
        self.parent_constructed();

        // Setup
        let obj = self.obj();
        obj.setup_software_titles();
        obj.setup_callbacks();
        obj.setup_factory();
        let dialog = simple_dialog::SimpleDialog::new();
        dialog.show();
    }
}

// Trait shared by all widgets
impl WidgetImpl for Window {}

// Trait shared by all windows
impl WindowImpl for Window {}

// Trait shared by all application windows
impl ApplicationWindowImpl for Window {}
