use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};
use ui_components::string_list_view::StringListItem;

#[derive(Debug)]
pub struct CorePickerDialog {
    file_set_id: i64,
    cores_wrapper: TypedListView<StringListItem<String>, gtk::SingleSelection>,
    selected_core: Option<String>,
}

pub struct CorePickerInit;

#[derive(Debug)]
pub enum CorePickerMsg {
    Show { cores: Vec<String>, file_set_id: i64 },
    SelectionChanged,
    LaunchClicked,
    Cancelled,
}

#[derive(Debug)]
pub enum CorePickerOutput {
    CoreChosen { core_name: String, file_set_id: i64 },
    Cancelled,
}

#[relm4::component(pub)]
impl Component for CorePickerDialog {
    type Init = CorePickerInit;
    type Input = CorePickerMsg;
    type Output = CorePickerOutput;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Window {
            set_modal: true,
            set_default_width: 400,
            set_default_height: 300,
            set_title: Some("Select Libretro Core"),
            connect_close_request[sender] => move |_| {
                sender.input(CorePickerMsg::Cancelled);
                glib::Propagation::Proceed
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,

                gtk::Label {
                    set_label: "Select a libretro core:",
                    set_xalign: 0.0,
                },

                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    cores_view -> gtk::ListView {},
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 6,
                    set_halign: gtk::Align::End,

                    gtk::Button {
                        set_label: "Launch",
                        #[watch]
                        set_sensitive: model.selected_core.is_some(),
                        connect_clicked => CorePickerMsg::LaunchClicked,
                    },
                    gtk::Button {
                        set_label: "Cancel",
                        connect_clicked => CorePickerMsg::Cancelled,
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let cores_wrapper: TypedListView<StringListItem<String>, gtk::SingleSelection> =
            TypedListView::new();

        cores_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |_| sender.input(CorePickerMsg::SelectionChanged)
            ));

        let model = CorePickerDialog {
            file_set_id: 0,
            cores_wrapper,
            selected_core: None,
        };

        let cores_view = &model.cores_wrapper.view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            CorePickerMsg::Show { cores, file_set_id } => {
                self.file_set_id = file_set_id;
                self.selected_core = None;
                self.cores_wrapper.clear();
                self.cores_wrapper
                    .extend_from_iter(cores.into_iter().map(|name| StringListItem { name }));
                root.present();
            }
            CorePickerMsg::SelectionChanged => {
                let idx = self.cores_wrapper.selection_model.selected();
                self.selected_core = self
                    .cores_wrapper
                    .get_visible(idx)
                    .map(|item| item.borrow().name.clone());
            }
            CorePickerMsg::LaunchClicked => {
                if let Some(core_name) = self.selected_core.clone() {
                    sender
                        .output(CorePickerOutput::CoreChosen {
                            core_name,
                            file_set_id: self.file_set_id,
                        })
                        .unwrap_or_default();
                    root.hide();
                }
            }
            CorePickerMsg::Cancelled => {
                sender.output(CorePickerOutput::Cancelled).unwrap_or_default();
                root.hide();
            }
        }
    }
}
