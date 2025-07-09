use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, Button, CompositeTemplate, Entry};

#[derive(CompositeTemplate, Default)]
#[template(resource = "/org/zorrokid/emufiles/release_form.ui")]
pub struct ReleaseFormWindow {
    #[template_child(id = "name_entry")]
    pub name_entry: TemplateChild<Entry>,
    #[template_child(id = "save_button")]
    pub save_button: TemplateChild<Button>,
}

#[glib::object_subclass]
impl ObjectSubclass for ReleaseFormWindow {
    const NAME: &'static str = "ReleaseFormWindow";
    type Type = super::ReleaseFormWindow;
    type ParentType = gtk::Window;

    fn class_init(klass: &mut Self::Class) {
        Self::bind_template(klass);
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ReleaseFormWindow {
    fn constructed(&self) {
        self.parent_constructed();
        self.save_button.connect_clicked(|_| {
            println!("Saving release...");
        });
    }
}
impl WidgetImpl for ReleaseFormWindow {}
impl WindowImpl for ReleaseFormWindow {}
