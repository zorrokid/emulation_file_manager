use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::SoftwareTitleListModel;

use crate::defaults::{DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SoftwareTitleSelectWidget {
    // TODO: software_titles are also in the parent widget
    // - here we need them for the pick list
    software_titles: Vec<SoftwareTitleListModel>,
    // The currently selected software title for the pick list
    selected_software_title: Option<SoftwareTitleListModel>,
}

#[derive(Debug, Clone)]
pub enum SoftwareTitleSelectWidgetMessage {
    Reset,
    SoftwareTitleSelected(SoftwareTitleListModel),
    SetSoftwareTitles(Vec<SoftwareTitleListModel>),
}

impl SoftwareTitleSelectWidget {
    pub fn new() -> Self {
        Self {
            software_titles: vec![],
            selected_software_title: None,
        }
    }

    pub fn update(
        &mut self,
        message: SoftwareTitleSelectWidgetMessage,
    ) -> Task<SoftwareTitleSelectWidgetMessage> {
        match message {
            SoftwareTitleSelectWidgetMessage::SoftwareTitleSelected(software_title) => Task::done(
                SoftwareTitleSelectWidgetMessage::SoftwareTitleSelected(software_title.clone()),
            ),
            SoftwareTitleSelectWidgetMessage::SetSoftwareTitles(software_titles) => {
                self.software_titles = software_titles;
                self.selected_software_title = None;
                Task::none()
            }
            SoftwareTitleSelectWidgetMessage::Reset => {
                self.software_titles.clear();
                self.selected_software_title = None;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> iced::Element<SoftwareTitleSelectWidgetMessage> {
        let software_title_select = pick_list(
            self.software_titles.as_slice(),
            self.selected_software_title.clone(),
            SoftwareTitleSelectWidgetMessage::SoftwareTitleSelected,
        );
        let label = text!("Select software title").width(DEFAULT_LABEL_WIDTH);
        row![label, software_title_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
