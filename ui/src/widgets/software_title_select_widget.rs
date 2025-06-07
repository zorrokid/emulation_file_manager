use iced::{
    alignment::Vertical,
    widget::{pick_list, row, text},
    Task,
};
use service::view_models::SoftwareTitleListModel;

use crate::defaults::{
    DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_PICKER_WIDTH, DEFAULT_SPACING,
};

pub struct SoftwareTitleSelectWidget {
    // The currently selected software title for the pick list
    selected_software_title: Option<SoftwareTitleListModel>,
}

#[derive(Debug, Clone)]
pub enum SoftwareTitleSelectWidgetMessage {
    Reset,
    SoftwareTitleSelected(SoftwareTitleListModel),
}

impl SoftwareTitleSelectWidget {
    pub fn new() -> Self {
        Self {
            selected_software_title: None,
        }
    }

    pub fn update(
        &mut self,
        message: SoftwareTitleSelectWidgetMessage,
    ) -> Task<SoftwareTitleSelectWidgetMessage> {
        match message {
            SoftwareTitleSelectWidgetMessage::SoftwareTitleSelected(software_title) => {
                self.selected_software_title = Some(software_title.clone());
            }
            SoftwareTitleSelectWidgetMessage::Reset => {
                self.selected_software_title = None;
            }
        }
        Task::none()
    }

    pub fn view<'a>(
        &self,
        software_titles: &'a [SoftwareTitleListModel],
    ) -> iced::Element<'a, SoftwareTitleSelectWidgetMessage> {
        let software_title_select = pick_list(
            software_titles,
            self.selected_software_title.clone(),
            SoftwareTitleSelectWidgetMessage::SoftwareTitleSelected,
        )
        .width(DEFAULT_PICKER_WIDTH)
        .placeholder("Select software title");

        let label = text!("Software title").width(DEFAULT_LABEL_WIDTH);
        row![label, software_title_select]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
