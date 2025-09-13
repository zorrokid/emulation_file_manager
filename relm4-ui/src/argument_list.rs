use core_types::ArgumentType;
use relm4::{
    Component, ComponentParts, ComponentSender,
    gtk::{
        self,
        prelude::{ButtonExt, EntryBufferExtManual, EntryExt, OrientableExt, WidgetExt},
    },
    typed_view::list::TypedListView,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgumentListItem {
    pub argument: ArgumentType,
}

pub struct ListItemWidgets {
    label: gtk::Label,
}

impl relm4::typed_view::list::RelmListItem for ArgumentListItem {
    type Root = gtk::Box;
    type Widgets = ListItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, ListItemWidgets) {
        relm4::view! {
            my_box = gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                #[name = "label"]
                gtk::Label,
            }
        }

        let widgets = ListItemWidgets { label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let ListItemWidgets { label } = widgets;
        label.set_label(self.argument.to_string().as_str());
    }
}

#[derive(Debug)]
pub enum ArgumentListMsg {
    SetArguments(Vec<ArgumentType>),
    MoveArgumentUp,
    MoveArgumentDown,
    Delete,
    AddArgument(String),
}

#[derive(Debug)]
pub enum ArgumentListOutputMsg {
    ArgumentsChanged(Vec<ArgumentType>),
}

#[derive(Debug)]
pub struct ArgumentList {
    list_view_wrapper: TypedListView<ArgumentListItem, gtk::SingleSelection>,
    is_active: bool,
}

#[relm4::component(pub)]
impl Component for ArgumentList {
    type Input = ArgumentListMsg;
    type Output = ArgumentListOutputMsg;
    type CommandOutput = ();
    type Init = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Label {
                set_label: "Add flag command line argument",
            },

            gtk::Entry {
                #[watch]
                set_sensitive: model.is_active,
                connect_activate[sender] => move |entry| {
                    let buffer = entry.buffer();
                    sender.input(ArgumentListMsg::AddArgument(buffer.text().into()));
                    buffer.delete_text(0, None);
                }
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                gtk::ScrolledWindow {
                    set_hscrollbar_policy: gtk::PolicyType::Never,
                    set_min_content_height: 360,
                    set_vexpand: true,
                    set_hexpand: true,

                    #[local_ref]
                    arguments_list_view -> gtk::ListView{}

                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    gtk::Button {
                        set_label: "Up",
                        #[watch]
                        set_sensitive: model.list_view_wrapper.len() > 1,
                        connect_clicked => ArgumentListMsg::MoveArgumentUp,
                    },
                     gtk::Button {
                        set_label: "Delete",
                        #[watch]
                        set_sensitive: !model.list_view_wrapper.is_empty(),
                        connect_clicked => ArgumentListMsg::Delete,
                    },
                   gtk::Button {
                        set_label: "Down",
                        #[watch]
                        set_sensitive: model.list_view_wrapper.len() > 1,
                        connect_clicked => ArgumentListMsg::MoveArgumentDown,
                    },
                },

            },
        },
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            ArgumentListMsg::AddArgument(argument_string) => {
                let argument = ArgumentType::try_from(argument_string.as_str());
                match argument {
                    Ok(argument) => {
                        self.list_view_wrapper.append(ArgumentListItem { argument });
                        self.emit_arguments_changed(&sender);
                    }
                    Err(e) => {
                        eprintln!("Error parsing command line argument: {}", e);
                    }
                }
            }
            ArgumentListMsg::MoveArgumentUp => {
                let selected_index = self.get_selected_index();
                if self.move_up(selected_index) != selected_index {
                    self.emit_arguments_changed(&sender);
                }
            }
            ArgumentListMsg::MoveArgumentDown => {
                let index = self.get_selected_index();
                if self.move_down(index) != index {
                    self.emit_arguments_changed(&sender);
                }
            }
            ArgumentListMsg::Delete => {
                let index = self.get_selected_index();
                if index < self.list_view_wrapper.len() {
                    self.list_view_wrapper.remove(index);
                    self.emit_arguments_changed(&sender);
                }
            }
            ArgumentListMsg::SetArguments(arguments) => {
                self.clear();
                self.extend_from_arguments(arguments.into_iter());
                self.emit_arguments_changed(&sender);
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let list_view_wrapper: TypedListView<ArgumentListItem, gtk::SingleSelection> =
            TypedListView::new();

        let model = Self {
            list_view_wrapper,
            is_active: true,
        };
        let arguments_list_view = &model.list_view_wrapper.view;
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }
}

impl ArgumentList {
    pub fn collect_arguments(&self) -> Vec<ArgumentType> {
        let mut arguments = Vec::new();
        for i in 0..self.list_view_wrapper.len() {
            if let Some(item) = self.list_view_wrapper.get(i) {
                arguments.push(item.borrow().argument.clone());
            }
        }
        arguments
    }

    fn emit_arguments_changed(&self, sender: &ComponentSender<Self>) {
        let arguments = self.collect_arguments();
        let res = sender.output(ArgumentListOutputMsg::ArgumentsChanged(arguments));
        if let Err(e) = res {
            eprintln!("Failed to send output message: {:?}", e);
        }
    }
    pub fn clear(&mut self) {
        self.list_view_wrapper.clear();
    }
    pub fn extend_from_arguments(&mut self, arguments: impl Iterator<Item = ArgumentType>) {
        let argument_list_items = arguments.map(|arg| ArgumentListItem { argument: arg });
        self.list_view_wrapper.extend_from_iter(argument_list_items);
    }
    pub fn get_selected_index(&self) -> u32 {
        self.list_view_wrapper.selection_model.selected()
    }

    pub fn move_up(&mut self, index: u32) -> u32 {
        if index > 0 {
            if let Some(item) = self.list_view_wrapper.get(index) {
                let new_index = index - 1;
                let argument = item.borrow().argument.clone();
                self.list_view_wrapper.remove(index);
                self.list_view_wrapper
                    .insert(new_index, ArgumentListItem { argument });
                self.list_view_wrapper
                    .selection_model
                    .set_selected(new_index);
                return new_index;
            }
        }
        index
    }

    pub fn move_down(&mut self, index: u32) -> u32 {
        if index < self.list_view_wrapper.len() - 1 {
            if let Some(item) = self.list_view_wrapper.get(index) {
                let new_index = index + 1;
                let argument = item.borrow().argument.clone();
                self.list_view_wrapper.remove(index);
                self.list_view_wrapper
                    .insert(new_index, ArgumentListItem { argument });
                self.list_view_wrapper
                    .selection_model
                    .set_selected(new_index);
                return new_index;
            }
        }
        index
    }
}
