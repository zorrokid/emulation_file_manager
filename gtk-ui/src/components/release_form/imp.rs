use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, Button, CompositeTemplate, DropDown, Entry};

use crate::objects::system::SystemObject;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/org/zorrokid/emufiles/release_form.ui")]
pub struct ReleaseFormWindow {
    #[template_child(id = "name_entry")]
    pub name_entry: TemplateChild<Entry>,
    #[template_child(id = "system_dropdown")]
    pub system_dropdown: TemplateChild<DropDown>,
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

        let systems = vec![
            SystemObject::new(1, "Commodore 64".to_string()),
            SystemObject::new(2, "Nintendo Entertainment System".to_string()),
            SystemObject::new(3, "Sinclair ZX Spectrum".to_string()),
        ];

        let list_store = gtk::gio::ListStore::new::<SystemObject>();
        for sys in systems {
            list_store.append(&sys);
        }
        let expr = gtk::PropertyExpression::new(
            SystemObject::static_type(),
            None::<gtk::Expression>,
            "name",
        );
        self.system_dropdown.set_expression(Some(&expr));
        self.system_dropdown.set_model(Some(&list_store));

        self.save_button.connect_clicked(|_| {
            println!("Saving release...");
        });
    }
}
impl WidgetImpl for ReleaseFormWindow {}
impl WindowImpl for ReleaseFormWindow {}
