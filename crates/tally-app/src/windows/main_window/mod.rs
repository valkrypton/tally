//! The main window: a sidebar with Calendar, Activities, Export and Settings.

pub mod activities;
pub mod calendar;
pub mod export;
pub mod settings;

use iced::alignment::Vertical;
use iced::widget::{Space, button, column, container, row, text};
use iced::{Element, Length, Padding, Size};
use jiff::civil::Date;
use jiff::tz::TimeZone;
use tally_core::settings::Settings;

use self::activities::Activities;
use self::calendar::Calendar;
use self::export::Export;
use self::settings::SettingsView;
use super::KeyPress;
use super::widgets::status_line;
use crate::status::Status;
use crate::theme;

pub const SIZE: Size = Size::new(880.0, 640.0);
pub const MIN_SIZE: Size = Size::new(760.0, 560.0);
const SIDEBAR_WIDTH: f32 = 200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Calendar,
    Activities,
    Export,
    Settings,
}

#[derive(Debug, Clone)]
pub enum Message {
    Section(Section),
    Calendar(calendar::Message),
    Activities(activities::Message),
    Export(export::Message),
    Settings(settings::Message),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Switched section: reload its data.
    Opened(Section),
    Calendar(calendar::Outcome),
    Activities(activities::Outcome),
    Export(export::Outcome),
    Settings(settings::Outcome),
}

#[derive(Debug, Clone)]
pub struct MainWindow {
    pub section: Section,
    pub calendar: Calendar,
    pub activities: Activities,
    pub export: Export,
    pub settings: SettingsView,
    status: Status,
    /// The pending-writes warning (spec §7), shown above every section.
    notice: Option<String>,
}

impl MainWindow {
    pub fn new(today: Date, tz: TimeZone, settings: Settings, data_file: String) -> Self {
        Self {
            section: Section::Calendar,
            calendar: Calendar::new(today, tz),
            activities: Activities::default(),
            export: Export::new(today),
            settings: SettingsView::new(settings, data_file),
            status: Status {
                tracking: false,
                text: String::new(),
                figure: None,
            },
            notice: None,
        }
    }

    pub fn set_status(&mut self, status: Status, notice: Option<String>) {
        self.status = status;
        self.notice = notice;
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::Section(section) => {
                self.section = section;
                Some(Outcome::Opened(section))
            }
            Message::Calendar(m) => self.calendar.update(m).map(Outcome::Calendar),
            Message::Activities(m) => self.activities.update(m).map(Outcome::Activities),
            Message::Export(m) => self.export.update(m).map(Outcome::Export),
            Message::Settings(m) => self.settings.update(m).map(Outcome::Settings),
        }
    }

    /// ⌘1 to ⌘4 switch sections; other keys go to the open section.
    pub fn key(&mut self, press: &KeyPress) -> Option<Outcome> {
        if press.modifiers.command()
            && let iced::keyboard::Key::Character(c) = press.key.as_ref()
        {
            let section = match c {
                "1" => Some(Section::Calendar),
                "2" => Some(Section::Activities),
                "3" => Some(Section::Export),
                "4" => Some(Section::Settings),
                _ => None,
            };
            if let Some(section) = section {
                return self.update(Message::Section(section));
            }
        }
        match self.section {
            Section::Calendar => self.calendar.key(press).map(Outcome::Calendar),
            Section::Activities => self.activities.key(press).map(Outcome::Activities),
            Section::Settings => self.settings.key(press).map(Outcome::Settings),
            Section::Export => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let nav = |section: Section, label: &'static str| {
            let selected = self.section == section;
            button(text(label).size(14).font(if selected {
                theme::UI_SEMIBOLD
            } else {
                theme::UI
            }))
            .width(Length::Fill)
            .height(36)
            .padding(Padding::from([9.0, 10.0]))
            .style(theme::nav_item(selected))
            .on_press(Message::Section(section))
        };
        let brand = row![
            container(Space::new())
                .width(8)
                .height(8)
                .style(theme::brand_mark),
            text("tally").font(theme::UI_BOLD).size(18),
        ]
        .spacing(theme::S_SM)
        .align_y(Vertical::Center);
        let sidebar = container(
            column![
                container(brand).padding(Padding {
                    top: 4.0,
                    right: 10.0,
                    bottom: 16.0,
                    left: 10.0
                }),
                nav(Section::Calendar, "Calendar"),
                nav(Section::Activities, "Activities"),
                nav(Section::Export, "Export"),
                nav(Section::Settings, "Settings"),
                Space::new().height(Length::Fill),
                container(status_line(&self.status)).padding(Padding::from([0.0, 10.0])),
            ]
            .spacing(theme::S_XS),
        )
        .width(SIDEBAR_WIDTH)
        .height(Length::Fill)
        .padding(Padding::from([16.0, 10.0]))
        .style(theme::deep_surface);

        let section: Element<'_, Message> = match self.section {
            Section::Calendar => self.calendar.view().map(Message::Calendar),
            Section::Activities => self.activities.view().map(Message::Activities),
            Section::Export => self.export.view().map(Message::Export),
            Section::Settings => self.settings.view().map(Message::Settings),
        };
        let mut content = column![].spacing(theme::S_LG).height(Length::Fill);
        if let Some(notice) = &self.notice {
            content = content.push(
                container(theme::small(notice.clone()).color(theme::ALERT))
                    .padding(Padding::from([10.0, 12.0]))
                    .width(Length::Fill)
                    .style(theme::alert_notice),
            );
        }
        content = content.push(section);

        container(row![
            sidebar,
            container(Space::new())
                .width(1)
                .height(Length::Fill)
                .style(theme::hairline),
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(Padding::from([20.0, 24.0])),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::main_surface)
        .into()
    }
}

/// The main-window notice for writes that haven't reached the database.
pub fn pending_notice(unsaved: usize, error: Option<&str>) -> Option<String> {
    let error = error?;
    let what = match unsaved {
        0 => String::from("tally couldn't save its state"),
        1 => String::from("tally couldn't save 1 entry"),
        n => format!("tally couldn't save {n} entries"),
    };
    Some(format!(
        "{what} ({error}). It keeps them in memory and retries every 30 seconds."
    ))
}

#[cfg(test)]
mod tests;
