use iced::{
    alignment::Vertical,
    widget::{button, pick_list, row, text},
    Task,
};
use service::view_models::SoftwareTitleListModel;

use crate::defaults::{DEFAULT_LABEL_WIDTH, DEFAULT_PADDING, DEFAULT_SPACING};

pub struct SoftwareTitleFilterWidget {
    software_titles: Vec<SoftwareTitleListModel>,
    selected_software_title: Option<SoftwareTitleListModel>,
}

#[derive(Debug, Clone)]
pub enum SoftwareTitleFilterWidgetMessage {
    SetSoftwareTitles(Vec<SoftwareTitleListModel>),
    SetSelectedSoftwareTitle(Option<i64>),
    // local messages
    SoftwareTitleSelected(SoftwareTitleListModel),
    ClearSelection,
}

impl SoftwareTitleFilterWidget {
    pub fn new() -> Self {
        Self {
            software_titles: vec![],
            selected_software_title: None,
        }
    }

    pub fn update(
        &mut self,
        message: SoftwareTitleFilterWidgetMessage,
    ) -> Task<SoftwareTitleFilterWidgetMessage> {
        match message {
            SoftwareTitleFilterWidgetMessage::SoftwareTitleSelected(software_title) => {
                self.selected_software_title = Some(software_title.clone());
                Task::done(SoftwareTitleFilterWidgetMessage::SetSelectedSoftwareTitle(
                    Some(software_title.id),
                ))
            }
            SoftwareTitleFilterWidgetMessage::SetSoftwareTitles(software_titles) => {
                self.software_titles = software_titles;
                self.selected_software_title = None;
                Task::none()
            }
            SoftwareTitleFilterWidgetMessage::ClearSelection => {
                self.selected_software_title = None;
                Task::done(SoftwareTitleFilterWidgetMessage::SetSelectedSoftwareTitle(
                    None,
                ))
            }
            _ => Task::none(),
        }
    }

    pub fn view(&self) -> iced::Element<SoftwareTitleFilterWidgetMessage> {
        let software_title_select = pick_list(
            self.software_titles.as_slice(),
            self.selected_software_title.clone(),
            SoftwareTitleFilterWidgetMessage::SoftwareTitleSelected,
        );
        let label = text!("Select software title").width(DEFAULT_LABEL_WIDTH);
        let clear_button =
            button("Clear").on_press(SoftwareTitleFilterWidgetMessage::ClearSelection);
        row![label, software_title_select, clear_button]
            .spacing(DEFAULT_SPACING)
            .padding(DEFAULT_PADDING)
            .align_y(Vertical::Center)
            .into()
    }
}
