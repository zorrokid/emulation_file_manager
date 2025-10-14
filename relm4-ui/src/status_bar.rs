use gtk::prelude::*;
use gtk::{Box as GtkBox, Label, Orientation, ProgressBar};
use relm4::prelude::*;

#[derive(Debug)]
pub enum StatusBarMsg {
    SetStatus(String),
    StartProgress { total: usize },
    UpdateProgress { done: usize, total: usize },
    Finish,
    Fail(String),
}

#[tracker::track]
pub struct StatusBarModel {
    status_text: String,
    total: usize,
    done: usize,
    syncing: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for StatusBarModel {
    type Init = ();
    type Input = StatusBarMsg;
    type Output = ();

    view! {
        #[root]
        GtkBox {
            set_orientation: Orientation::Horizontal,
            set_spacing: 8,
            set_margin_all: 6,

            Label {
                #[watch]
                set_label: &model.status_text,
            },

            ProgressBar {
                #[watch]
                set_visible: model.syncing,
                #[watch]
                set_fraction: if model.total > 0 {
                    model.done as f64 / model.total as f64
                } else {
                    0.0
                },
                set_hexpand: true,
            }
        }
    }

    fn init(_init: (), root: Self::Root, sender: ComponentSender<Self>) -> ComponentParts<Self> {
        let model = StatusBarModel {
            status_text: "Ready.".into(),
            total: 0,
            done: 0,
            syncing: false,
            tracker: 0,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: StatusBarMsg, _sender: ComponentSender<Self>) {
        self.reset();

        match msg {
            StatusBarMsg::SetStatus(text) => {
                self.status_text = text;
            }
            StatusBarMsg::StartProgress { total } => {
                self.status_text = "Syncing files...".into();
                self.total = total;
                self.done = 0;
                self.syncing = true;
            }
            StatusBarMsg::UpdateProgress { done, total } => {
                self.done = done;
                self.total = total;
                self.status_text = format!("Synced {done}/{total} files");
            }
            StatusBarMsg::Finish => {
                self.status_text = "Sync complete.".into();
                self.syncing = false;
            }
            StatusBarMsg::Fail(error) => {
                self.status_text = format!("Sync failed: {error}");
                self.syncing = false;
            }
        }
    }
}
