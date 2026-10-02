//! Shown when the database can't be opened at startup (spec §7). tally never starts a
//! fresh empty file in its place. Artboard B-Error.

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length, Padding, Size};

use super::widgets::{ghost_button, icon, primary_button};
use crate::{icons, theme};

pub const SIZE: Size = Size::new(440.0, 280.0);

#[derive(Debug, Clone)]
pub enum Message {
    Retry,
    OpenFolder,
    Quit,
}

#[derive(Debug, Clone)]
pub struct DataError {
    pub path: String,
    pub error: String,
}

impl DataError {
    pub fn view(&self) -> Element<'_, Message> {
        let tile = container(icon(icons::alert(), 18.0, theme::ALERT))
            .width(36)
            .height(36)
            .center(36)
            .style(theme::alert_tile);
        let heading = column![
            text("tally can't open its data file")
                .font(theme::UI_SEMIBOLD)
                .size(16),
            theme::small(
                "Nothing has been tracked since this happened. Your existing data is \
                 untouched; tally won't start a new empty file in its place."
            )
            .color(theme::TEXT_MUTED),
        ]
        .spacing(6);
        let details = container(
            column![
                text(self.path.clone()).font(theme::MONO).size(12),
                text(self.error.clone())
                    .font(theme::MONO)
                    .size(12)
                    .color(theme::ALERT),
            ]
            .spacing(theme::S_XS),
        )
        .padding(Padding::from([10.0, 12.0]))
        .width(Length::Fill)
        .style(theme::raised_panel);
        let open_folder = button(theme::small("Open folder"))
            .height(40)
            .padding(Padding::from([11.0, 12.0]))
            .style(theme::button_outlined)
            .on_press(Message::OpenFolder);
        let actions = row![
            ghost_button("Quit", None, Some(Message::Quit)),
            Space::new().width(Length::Fill),
            open_folder,
            primary_button(String::from("Try again"), None, None, Some(Message::Retry)),
        ]
        .spacing(theme::S_SM)
        .align_y(Vertical::Center);
        container(column![
            container(row![tile, heading].spacing(14)).padding(Padding {
                top: 20.0,
                right: 20.0,
                bottom: 0.0,
                left: 20.0
            }),
            container(details).padding(Padding {
                top: 16.0,
                right: 20.0,
                bottom: 0.0,
                left: 20.0
            }),
            Space::new().height(Length::Fill),
            container(actions).padding(Padding::from([12.0, 16.0])),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::popup_frame)
        .into()
    }
}
