use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/org/zorrokid/emufiles/system_dialog.ui")]
pub struct SystemDialog {
    #[template_child(id = "system_name_entry")]
    pub system_name_entry: TemplateChild<gtk::Entry>,
}

#[glib::object_subclass]
impl ObjectSubclass for SystemDialog {
    const NAME: &'static str = "SystemDialog";
    type Type = super::SystemDialog;
    type ParentType = gtk::Dialog;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for SystemDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.connect_close_request(|dialog| {
            dialog.hide();
            gtk::glib::Propagation::Stop
        });
    }
}

impl WidgetImpl for SystemDialog {}
impl WindowImpl for SystemDialog {}
impl DialogImpl for SystemDialog {}
