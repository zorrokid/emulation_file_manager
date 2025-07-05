use crate::objects::repository_manager::RepositoryManagerObject;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

#[derive(Default, gtk::CompositeTemplate)]
#[template(resource = "/org/zorrokid/emufiles/add_release_dialog.ui")]
pub struct AddReleaseDialog {
    #[template_child(id = "some_widget")]
    pub some_widget: TemplateChild<gtk::Entry>,
    repository_manager: std::cell::OnceCell<RepositoryManagerObject>,
}

#[glib::object_subclass]
impl ObjectSubclass for AddReleaseDialog {
    const NAME: &'static str = "AddReleaseDialog";
    type Type = super::AddReleaseDialog;
    type ParentType = gtk::Dialog;
}

impl ObjectImpl for AddReleaseDialog {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.connect_close_request(|dialog| {
            dialog.hide();
            gtk::glib::Propagation::Stop
        });
    }

    fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "repository-manager" => {
                let repo_manager = value
                    .get::<RepositoryManagerObject>()
                    .expect("type checked upstream");
                self.repository_manager.set(repo_manager).ok();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "repository-manager" => self.repository_manager.get().cloned().to_value(),
            _ => unimplemented!(),
        }
    }
}

impl WidgetImpl for AddReleaseDialog {}
impl WindowImpl for AddReleaseDialog {}
impl DialogImpl for AddReleaseDialog {}
