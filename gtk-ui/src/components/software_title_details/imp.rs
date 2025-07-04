use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/org/zorrokid/emufiles/software_title_details.ui")]
pub struct SoftwareTitleDetails {
    #[template_child(id = "title_label")]
    pub title_label: TemplateChild<gtk::Label>,
    #[template_child(id = "releases_grid")]
    pub releases_grid: TemplateChild<gtk::GridView>,
    pub releases_model: std::cell::OnceCell<gtk::NoSelection>,
}

#[glib::object_subclass]
impl ObjectSubclass for SoftwareTitleDetails {
    const NAME: &'static str = "SoftwareTitleDetails";
    type Type = super::SoftwareTitleDetails;
    type ParentType = gtk::Box;
    type Interfaces = ();
    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
    }
    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

// Trait shared by all GObjects
impl ObjectImpl for SoftwareTitleDetails {}

// Trait shared by all widgets
impl WidgetImpl for SoftwareTitleDetails {}

// Trait shared by all boxes
impl BoxImpl for SoftwareTitleDetails {}
