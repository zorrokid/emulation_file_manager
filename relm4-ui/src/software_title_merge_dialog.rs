use relm4::{
    Component, ComponentParts, ComponentSender, RelmWidgetExt,
    gtk::{
        self,
        glib::{self, clone},
        prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};

use crate::list_item::ListItem;

#[derive(Debug)]
pub struct SoftwareTitleMergeDialog {
    software_titles_to_merge: Vec<ListItem>,
    selected_list_item: Option<ListItem>,
    list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection>,
}

#[derive(Debug)]
pub enum SoftwareTitleMergeDialogMsg {
    Show {
        software_titles_to_merge: Vec<ListItem>,
    },
    Hide,
    Select,
    SelectionChanged,
}

#[derive(Debug)]
pub enum SoftwareTitleMergeDialogOutputMsg {
    Selected(ListItem),
}

#[relm4::component(pub)]
impl Component for SoftwareTitleMergeDialog {
    type Init = ();
    type Input = SoftwareTitleMergeDialogMsg;
    type Output = SoftwareTitleMergeDialogOutputMsg;
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Window{
            set_default_width: 400,
            set_default_height: 600,
            set_title: Some("Select Base Software Title"),
            connect_close_request[sender] => move |_| {
                sender.input(SoftwareTitleMergeDialogMsg::Hide);
                glib::Propagation::Proceed
            },
             gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 10,
                set_spacing: 10,
                gtk::ScrolledWindow {
                    set_vexpand: true,
                    #[local_ref]
                    software_titles_list_view -> gtk::ListView {}
                },
                gtk::Button {
                    set_label: "Cancel",
                    connect_clicked[sender] => move |_| {
                        sender.input(SoftwareTitleMergeDialogMsg::Hide);
                    }
                },
                gtk::Button {
                    set_label: "Select",
                    connect_clicked[sender] => move |_| {
                        sender.input(SoftwareTitleMergeDialogMsg::Select);
                    }
                }

             }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ListItem, gtk::SingleSelection> =
            TypedListView::with_sorting();

        list_view_wrapper
            .selection_model
            .connect_selected_notify(clone!(
                #[strong]
                sender,
                move |_| {
                    sender.input(SoftwareTitleMergeDialogMsg::SelectionChanged);
                }
            ));

        let model = SoftwareTitleMergeDialog {
            software_titles_to_merge: Vec::new(),
            selected_list_item: None,
            list_view_wrapper,
        };

        let software_titles_list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, root: &Self::Root) {
        match msg {
            SoftwareTitleMergeDialogMsg::Select => {
                if let Some(software_title) = self.get_selected_software_title_list_model() {
                    sender
                        .output(SoftwareTitleMergeDialogOutputMsg::Selected(software_title))
                        .unwrap_or_else(|e| {
                            tracing::error!(
                                error = ?e,
                                "Failed to send software_title selection output"
                            );
                        });

                    sender.input(SoftwareTitleMergeDialogMsg::Hide);
                }
            }
            SoftwareTitleMergeDialogMsg::Show {
                software_titles_to_merge,
            } => {
                self.software_titles_to_merge = software_titles_to_merge;
                self.list_view_wrapper
                    .extend_from_iter(self.software_titles_to_merge.clone());
                root.show();
            }
            SoftwareTitleMergeDialogMsg::Hide => {
                self.selected_list_item = None;
                self.list_view_wrapper.clear();
                root.hide();
            }
            SoftwareTitleMergeDialogMsg::SelectionChanged => {
                self.selected_list_item = self.get_selected_list_item();
            }
        }
    }
}

impl SoftwareTitleMergeDialog {
    fn get_selected_list_item(&self) -> Option<ListItem> {
        let selected_index = self.list_view_wrapper.selection_model.selected();
        if let Some(item) = self.list_view_wrapper.get_visible(selected_index) {
            let item = item.borrow();
            Some(item.clone())
        } else {
            None
        }
    }
    fn get_selected_software_title_list_model(&self) -> Option<ListItem> {
        if let Some(selected_item) = self.get_selected_list_item() {
            Some(ListItem {
                id: selected_item.id,
                name: selected_item.name,
            })
        } else {
            None
        }
    }
}
