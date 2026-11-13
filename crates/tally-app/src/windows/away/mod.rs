//! "Welcome back": what was the time before you left, and while you were away?
//! Artboard B-Away.

use iced::alignment::Vertical;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text::LineHeight;
use iced::widget::{Id, Space, button, column, container, row, text, text_input};
use iced::{Element, Length, Padding, Size};
use jiff::Timestamp;
use jiff::tz::TimeZone;
use tally_core::ActivityName;

use super::KeyPress;
use super::widgets::{self, footer, ghost_button, primary_button};
use crate::{format, theme};

pub const SIZE: Size = Size::new(400.0, 420.0);
const CHIPS: usize = 3;

pub fn before_id() -> Id {
    Id::new("away-before")
}

pub fn away_id() -> Id {
    Id::new("away-during")
}

#[derive(Debug, Clone)]
pub struct Setup {
    pub cursor: Timestamp,
    pub left_at: Timestamp,
    pub back_at: Timestamp,
    pub previous: Option<ActivityName>,
    pub suggestions: Vec<ActivityName>,
    pub tz: TimeZone,
}

#[derive(Debug, Clone)]
pub enum Message {
    Before(String),
    During(String),
    /// A quick-pick chip; `None` is "Don't count it".
    Chip(Option<usize>),
    Submit,
    Discard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `None` leaves that time unrecorded.
    Resolve {
        before: Option<ActivityName>,
        away: Option<ActivityName>,
    },
}

#[derive(Debug, Clone)]
pub struct Away {
    setup: Setup,
    before: String,
    during: String,
}

impl Away {
    pub fn new(setup: Setup) -> Self {
        let before = setup
            .previous
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        Self {
            setup,
            before,
            during: String::new(),
        }
    }

    /// The "before you left" row only shows when there is time before leaving.
    pub fn has_before(&self) -> bool {
        self.setup.left_at > self.setup.cursor
    }

    /// The field that gets focus when the prompt opens.
    pub fn first_field(&self) -> Id {
        if self.has_before() {
            before_id()
        } else {
            away_id()
        }
    }

    fn chips(&self) -> Vec<&ActivityName> {
        self.setup.suggestions.iter().take(CHIPS).collect()
    }

    /// Parsed answers; an empty field means "don't count it".
    pub fn answers(&self) -> (Option<ActivityName>, Option<ActivityName>) {
        let before = if self.has_before() {
            ActivityName::parse(&self.before).ok()
        } else {
            None
        };
        (before, ActivityName::parse(&self.during).ok())
    }

    /// Seconds that "Log" would record.
    pub fn logged_seconds(&self) -> i64 {
        let (before, away) = self.answers();
        let mut seconds = 0;
        if before.is_some() {
            seconds += self
                .setup
                .left_at
                .duration_since(self.setup.cursor)
                .as_secs();
        }
        if away.is_some() {
            seconds += self
                .setup
                .back_at
                .duration_since(self.setup.left_at)
                .as_secs();
        }
        seconds
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::Before(value) => self.before = value,
            Message::During(value) => self.during = value,
            Message::Chip(Some(index)) => {
                if let Some(activity) = self.chips().get(index) {
                    self.during = activity.to_string();
                }
            }
            Message::Chip(None) => self.during.clear(),
            Message::Submit => {
                let (before, away) = self.answers();
                return Some(Outcome::Resolve { before, away });
            }
            Message::Discard => {
                return Some(Outcome::Resolve {
                    before: None,
                    away: None,
                });
            }
        }
        None
    }

    /// Esc discards both; ⌘1–3 pick a chip; ⌘4 is "Don't count it".
    pub fn key(&mut self, press: &KeyPress) -> Option<Outcome> {
        match press.key.as_ref() {
            Key::Named(Named::Escape) => self.update(Message::Discard),
            Key::Named(Named::Enter) if !press.captured => self.update(Message::Submit),
            Key::Character(c) if press.modifiers.command() => match c {
                "1" | "2" | "3" => {
                    let index = c.parse::<usize>().ok()? - 1;
                    self.update(Message::Chip(Some(index)))
                }
                "4" => self.update(Message::Chip(None)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let tz = &self.setup.tz;
        let away_secs = self
            .setup
            .back_at
            .duration_since(self.setup.left_at)
            .as_secs();
        let header = column![
            row![
                theme::title("Welcome back"),
                Space::new().width(Length::Fill),
                text(format!("away {}", format::short_duration(away_secs)))
                    .font(theme::MONO)
                    .size(12)
                    .color(theme::TEXT_MUTED),
            ]
            .align_y(Vertical::Center),
            theme::small(format!(
                "No keyboard or mouse from {} to {}. What was that time?",
                format::clock(self.setup.left_at, tz),
                format::clock(self.setup.back_at, tz)
            ))
            .color(theme::TEXT_MUTED),
        ]
        .spacing(theme::S_XS);

        let mut fields = column![].spacing(theme::S_MD);
        if self.has_before() {
            fields = fields.push(field_block(
                "Before you left",
                format::window(self.setup.cursor, self.setup.left_at, tz),
                text_input("Leave empty to not count it", &self.before)
                    .id(before_id())
                    .on_input(Message::Before)
                    .on_submit(Message::Submit),
                false,
                None,
            ));
        }
        let chosen = self
            .chips()
            .iter()
            .position(|a| a.to_string() == self.during);
        let mut chips = row![].spacing(6);
        for (index, activity) in self.chips().into_iter().enumerate() {
            chips = chips.push(chip(
                activity.name.clone(),
                chosen == Some(index),
                false,
                Message::Chip(Some(index)),
            ));
        }
        chips = chips.push(chip(
            String::from("Don't count it"),
            self.during.trim().is_empty(),
            true,
            Message::Chip(None),
        ));
        fields = fields.push(field_block(
            "While away",
            format::window(self.setup.left_at, self.setup.back_at, tz),
            text_input("Type an activity, or pick one", &self.during)
                .id(away_id())
                .on_input(Message::During)
                .on_submit(Message::Submit),
            true,
            Some(chips.into()),
        ));

        let logged = self.logged_seconds();
        let label = if logged > 0 {
            format!("Log {}", format::duration(logged))
        } else {
            String::from("Done")
        };
        let actions = row![
            ghost_button("Discard both", Some("esc"), Some(Message::Discard)),
            Space::new().width(Length::Fill),
            primary_button(label, None, Some("enter"), Some(Message::Submit)),
        ]
        .spacing(theme::S_XS)
        .align_y(Vertical::Center);

        container(column![
            container(header).padding(Padding {
                top: 16.0,
                right: 16.0,
                bottom: 4.0,
                left: 16.0
            }),
            container(fields).padding(theme::S_MD).height(Length::Fill),
            footer(actions),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::popup_frame)
        .into()
    }
}

fn field_block<'a>(
    label: &'a str,
    window: String,
    input: iced::widget::TextInput<'a, Message>,
    active: bool,
    extra: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let look = widgets::look(false, active);
    let mut block = column![
        container(
            row![
                text(label).font(theme::UI_MEDIUM).size(13),
                Space::new().width(Length::Fill),
                text(window)
                    .font(theme::MONO)
                    .size(12)
                    .color(theme::TEXT_MUTED),
            ]
            .align_y(Vertical::Center)
        )
        .padding(Padding::from([0.0, 4.0])),
        input
            .font(theme::UI)
            .size(15)
            .line_height(LineHeight::Absolute(20.into()))
            .padding(Padding::from([12.0, 12.0]))
            .style(theme::prompt_field(look)),
    ]
    .spacing(6);
    if let Some(extra) = extra {
        block = block.push(container(extra).padding(Padding {
            top: 2.0,
            ..Padding::ZERO
        }));
    }
    block.into()
}

fn chip<'a>(label: String, selected: bool, dashed: bool, message: Message) -> Element<'a, Message> {
    button(text(label).size(13))
        .height(32)
        .padding(Padding::from([7.0, 12.0]))
        .style(theme::chip(selected, dashed))
        .on_press(message)
        .into()
}

#[cfg(test)]
mod tests;
