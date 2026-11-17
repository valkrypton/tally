//! The "What were you doing?" prompt: normal, late and new-activity variants plus the
//! More menu (keep for / ask again in / pause). Artboards B-Prompt, B-Prompt-Late,
//! B-Prompt-New and B-More.

use iced::alignment::Vertical;
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text::{LineHeight, Rich, Span};
use iced::widget::{Id, Space, button, checkbox, column, container, row, text, text_input};
use iced::{Element, Font, Length, Padding, Size};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use tally_core::ActivityName;
use tally_core::scheduler::KeepUntil;

use super::KeyPress;
use super::widgets::{self, footer, ghost_button, key_cap, primary_button};
use crate::{format, icons, theme};

pub const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 340.0;
const LATE_EXTRA: f32 = 40.0;

/// The answer field, focused and selected when the prompt opens.
pub fn input_id() -> Id {
    Id::new("prompt-input")
}

/// What the app knows when it opens (or refreshes) the prompt.
#[derive(Debug, Clone)]
pub struct Setup {
    /// Start of the interval being asked about (the scheduler's cursor).
    pub cursor: Timestamp,
    pub due_at: Timestamp,
    /// `Scheduler::late_coverage(now)`: `Some` once the answer is late.
    pub late: Option<SignedDuration>,
    pub previous: Option<ActivityName>,
    /// Non-archived activities, most recent first.
    pub suggestions: Vec<ActivityName>,
    /// End of today's working period, for "Until 18:00".
    pub day_end: Option<Timestamp>,
    pub tz: TimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowKind {
    /// The previous answer (shown first, labelled "previous").
    Previous,
    Recent,
    /// Matches what was typed.
    Match,
    /// What was typed, as a new activity.
    New,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub activity: ActivityName,
    pub kind: RowKind,
    /// Lowercase fragments to draw in amber.
    pub needles: Vec<String>,
}

/// One option in the More menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoreOption {
    Keep(KeepUntil),
    Snooze(SignedDuration),
    Pause(Option<SignedDuration>),
}

pub const MORE_OPTIONS: [MoreOption; 9] = [
    MoreOption::Keep(KeepUntil::For(SignedDuration::from_mins(30))),
    MoreOption::Keep(KeepUntil::For(SignedDuration::from_hours(1))),
    MoreOption::Keep(KeepUntil::For(SignedDuration::from_hours(2))),
    MoreOption::Keep(KeepUntil::EndOfDay),
    MoreOption::Snooze(SignedDuration::from_mins(5)),
    MoreOption::Snooze(SignedDuration::from_mins(10)),
    MoreOption::Pause(Some(SignedDuration::from_mins(30))),
    MoreOption::Pause(Some(SignedDuration::from_hours(1))),
    MoreOption::Pause(None),
];

/// "1h" is focused when the menu opens, as in the B-More artboard.
const DEFAULT_MORE_FOCUS: usize = 1;

#[derive(Debug, Clone)]
pub enum Message {
    Input(String),
    Submit,
    Pick(usize),
    Skip,
    ToggleMore,
    ToggleLate(bool),
    More(usize),
}

/// What the app should tell the scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Answer {
        activity: ActivityName,
        apply_late: bool,
    },
    Skip,
    Snooze(SignedDuration),
    Keep {
        activity: ActivityName,
        until: KeepUntil,
    },
    Pause(Option<SignedDuration>),
}

/// A key press, interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Escape,
    /// Enter while no field has focus (a field's own Enter is its `on_submit`).
    Submit,
    ToggleMore,
    ToggleLate,
    Up,
    Down,
    Pick(usize),
}

#[derive(Debug, Clone)]
pub struct Prompt {
    setup: Setup,
    input: String,
    /// False until the user types: the field still holds the pre-filled answer.
    edited: bool,
    selected: usize,
    apply_late: bool,
    /// `Some(focused option)` while the More menu is open.
    more: Option<usize>,
    invalid: bool,
}

impl Prompt {
    pub fn new(setup: Setup) -> Self {
        let input = setup
            .previous
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        Self {
            setup,
            input,
            edited: false,
            selected: 0,
            apply_late: true,
            more: None,
            invalid: false,
        }
    }

    /// Updates the times on each tick without losing what the user typed.
    pub fn refresh(&mut self, setup: Setup) {
        self.setup = setup;
        self.selected = self.selected.min(self.rows().len().saturating_sub(1));
    }

    /// The due time of the interval being asked about.
    pub fn due_at(&self) -> Timestamp {
        self.setup.due_at
    }

    pub fn window_size(&self) -> Size {
        let extra = if self.setup.late.is_some() {
            LATE_EXTRA
        } else {
            0.0
        };
        Size::new(WIDTH, HEIGHT + extra)
    }

    /// The late-answer notice takes one row's room.
    fn max_rows(&self) -> usize {
        4 - usize::from(self.setup.late.is_some())
    }

    pub fn rows(&self) -> Vec<Row> {
        let list = |max| {
            rows(
                &self.input,
                self.edited,
                self.setup.previous.as_ref(),
                &self.setup.suggestions,
                max,
            )
        };
        let max = self.max_rows();
        let all = list(max);
        // The "Category: Activity" hint takes the last row's place.
        if all.len() == max && all.iter().any(|r| r.kind == RowKind::New) {
            list(max - 1)
        } else {
            all
        }
    }

    #[cfg(test)]
    pub fn is_more_open(&self) -> bool {
        self.more.is_some()
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::Input(value) => {
                if let Some(index) = digit_pick(self.edited, &value, self.rows().len()) {
                    return self.pick(index);
                }
                self.input = value;
                self.edited = true;
                self.selected = 0;
                self.invalid = false;
                None
            }
            Message::Submit => match self.more {
                Some(focus) => self.choose_more(focus),
                None => self.answer(self.selected),
            },
            Message::Pick(index) => self.pick(index),
            Message::Skip => Some(Outcome::Skip),
            Message::ToggleMore => {
                self.more = match self.more {
                    Some(_) => None,
                    None => Some(DEFAULT_MORE_FOCUS),
                };
                None
            }
            Message::ToggleLate(on) => {
                self.apply_late = on;
                None
            }
            Message::More(index) => self.choose_more(index),
        }
    }

    pub fn key(&mut self, press: &KeyPress) -> Option<Outcome> {
        match key_command(press, self.more.is_some())? {
            Command::Escape if self.more.is_some() => {
                self.more = None;
                None
            }
            Command::Escape => Some(Outcome::Skip),
            Command::Submit => self.update(Message::Submit),
            Command::ToggleMore => self.update(Message::ToggleMore),
            Command::ToggleLate => {
                if self.setup.late.is_some() {
                    self.apply_late = !self.apply_late;
                }
                None
            }
            Command::Up => {
                match &mut self.more {
                    Some(focus) => *focus = focus.saturating_sub(1),
                    None => self.selected = self.selected.saturating_sub(1),
                }
                None
            }
            Command::Down => {
                match &mut self.more {
                    Some(focus) => *focus = (*focus + 1).min(MORE_OPTIONS.len() - 1),
                    None => {
                        self.selected = (self.selected + 1).min(self.rows().len().saturating_sub(1))
                    }
                }
                None
            }
            Command::Pick(index) => self.pick(index),
        }
    }

    fn pick(&mut self, index: usize) -> Option<Outcome> {
        if index < self.rows().len() {
            self.selected = index;
            self.answer(index)
        } else {
            None
        }
    }

    /// The activity an answer would log: the selected row, or what was typed.
    fn chosen(&self, index: usize) -> Option<ActivityName> {
        match self.rows().into_iter().nth(index) {
            Some(row) => Some(row.activity),
            None => ActivityName::parse(&self.input).ok(),
        }
    }

    fn answer(&mut self, index: usize) -> Option<Outcome> {
        match self.chosen(index) {
            Some(activity) => Some(Outcome::Answer {
                activity,
                apply_late: self.apply_late,
            }),
            None => {
                self.invalid = true;
                None
            }
        }
    }

    fn choose_more(&mut self, index: usize) -> Option<Outcome> {
        let option = *MORE_OPTIONS.get(index)?;
        match option {
            MoreOption::Snooze(duration) => Some(Outcome::Snooze(duration)),
            MoreOption::Pause(duration) => Some(Outcome::Pause(duration)),
            MoreOption::Keep(until) => match self.chosen(self.selected) {
                Some(activity) => Some(Outcome::Keep { activity, until }),
                None => {
                    self.more = None;
                    self.invalid = true;
                    None
                }
            },
        }
    }

    /// The time an answer would cover, for the header.
    fn covered(&self) -> (Timestamp, Timestamp) {
        let start = self.setup.cursor;
        match self.setup.late {
            Some(late) if self.apply_late => (start, start + late),
            _ => (start, self.setup.due_at),
        }
    }

    fn primary_label(&self, rows: &[Row]) -> String {
        if rows
            .get(self.selected)
            .is_some_and(|r| r.kind == RowKind::New)
        {
            String::from("Create & log")
        } else {
            match self.setup.late {
                Some(late) if self.apply_late => {
                    format!("Log {}", format::short_duration(late.as_secs()))
                }
                _ => String::from("Log it"),
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let tz = &self.setup.tz;
        let rows = self.rows();
        let primary_label = self.primary_label(&rows);
        let (start, end) = self.covered();

        let header = row![
            theme::title("What were you doing?"),
            Space::new().width(Length::Fill),
            text(format::window(start, end, tz))
                .font(theme::MONO)
                .size(12)
                .color(theme::TEXT_MUTED),
        ]
        .align_y(Vertical::Center);

        let mut body = column![container(header).padding(Padding {
            top: 16.0,
            right: 16.0,
            bottom: 12.0,
            left: 16.0
        })];

        if let Some(late) = self.setup.late {
            body = body.push(container(self.late_notice(late)).padding(Padding {
                top: 0.0,
                right: 12.0,
                bottom: 10.0,
                left: 12.0,
            }));
        }

        let look = widgets::look(self.invalid, self.more.is_none());
        let field = text_input("What are you doing?", &self.input)
            .id(input_id())
            .on_input(Message::Input)
            .on_submit(Message::Submit)
            .font(theme::UI)
            .size(16)
            .line_height(LineHeight::Absolute(20.into()))
            .padding(Padding::from([14.0, 12.0]))
            .style(theme::prompt_field(look));
        body = body.push(container(field).padding(Padding::from([0.0, 12.0])));

        match self.more {
            Some(focus) => {
                body = body.push(Space::new().height(Length::Fill));
                body =
                    body.push(container(self.more_menu(focus)).padding(Padding::from([0.0, 8.0])));
                body = body.push(Space::new().height(8));
            }
            None => {
                let has_new = rows.iter().any(|r| r.kind == RowKind::New);
                let list = column(
                    rows.into_iter()
                        .enumerate()
                        .map(|(i, r)| self.row_view(i, r)),
                )
                .spacing(theme::S_HAIR);
                body = body.push(container(list).padding(theme::S_SM).height(Length::Fill));
                if has_new {
                    body = body.push(container(new_hint()).padding(Padding {
                        top: 0.0,
                        right: 16.0,
                        bottom: 10.0,
                        left: 16.0,
                    }));
                }
            }
        }

        let more_button = button(
            container(
                row![theme::small("More"), key_cap(more_key())]
                    .spacing(theme::S_SM)
                    .align_y(Vertical::Center),
            )
            .height(Length::Fill)
            .align_y(Vertical::Center),
        )
        .height(40)
        .padding(Padding::from([0.0, 10.0]))
        .style(if self.more.is_some() {
            theme::button_ghost_on
        } else {
            theme::button_ghost
        })
        .on_press(Message::ToggleMore);

        let actions = row![
            ghost_button("Skip", Some("esc"), Some(Message::Skip)),
            more_button,
            Space::new().width(Length::Fill),
            primary_button(primary_label, None, Some("enter"), Some(Message::Submit)),
        ]
        .spacing(theme::S_XS)
        .align_y(Vertical::Center);
        body = body.push(footer(actions));

        container(body)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::popup_frame)
            .into()
    }

    fn late_notice(&self, late: SignedDuration) -> Element<'_, Message> {
        let tz = &self.setup.tz;
        let since = format::clock(self.setup.cursor, tz);
        let headline = row![
            text(format!(
                "Count all {} since {since}",
                format::short_duration(late.as_secs())
            ))
            .font(theme::UI_MEDIUM)
            .size(14)
            .color(theme::LAMP_AMBER_INK)
            .width(Length::Fill),
            key_cap(late_key()),
        ]
        .align_y(Vertical::Center);
        let lines = column![
            headline,
            text(format!(
                "Untick to log {} only and leave the rest empty.",
                format::window(self.setup.cursor, self.setup.due_at, tz)
            ))
            .size(12)
            .color(theme::LAMP_AMBER_INK_MUTED),
        ]
        .spacing(theme::S_HAIR)
        .width(Length::Fill);
        container(
            row![
                checkbox(self.apply_late)
                    .on_toggle(Message::ToggleLate)
                    .size(16)
                    .style(theme::checkbox_style),
                lines,
            ]
            .spacing(10)
            .align_y(Vertical::Top),
        )
        .padding(Padding::from([10.0, 12.0]))
        .width(Length::Fill)
        .style(theme::notice_wash)
        .into()
    }

    fn row_view(&self, index: usize, r: Row) -> Element<'_, Message> {
        let selected = index == self.selected;
        let chip: Element<'_, Message> = if r.kind == RowKind::New {
            container(widgets::icon(icons::plus(), 12.0, theme::CONSOLE_NIGHT))
                .width(20)
                .height(20)
                .center(20)
                .style(theme::number_chip(true))
                .into()
        } else {
            container(
                text((index + 1).to_string())
                    .font(if selected {
                        theme::MONO_BOLD
                    } else {
                        theme::MONO
                    })
                    .size(11),
            )
            .width(20)
            .height(20)
            .center(20)
            .style(theme::number_chip(selected))
            .into()
        };
        let mut line = row![chip, name_text(&r.activity, &r.needles)]
            .spacing(theme::S_MD)
            .align_y(Vertical::Center);
        let tag = match r.kind {
            RowKind::Previous => Some("previous"),
            RowKind::New => Some("new"),
            _ => None,
        };
        if let Some(tag) = tag {
            line = line.push(theme::label(tag));
        }
        button(line)
            .width(Length::Fill)
            .height(40)
            .padding(theme::S_SM)
            .style(theme::row_button(selected))
            .on_press(Message::Pick(index))
            .into()
    }

    fn more_menu(&self, focus: usize) -> Element<'_, Message> {
        let tz = &self.setup.tz;
        let name = self
            .chosen(self.selected)
            .map(|a| a.name)
            .unwrap_or_else(|| String::from("this"));
        let option = |index: usize, label: String, mono: bool| -> Element<'_, Message> {
            let content: Element<'_, Message> = match MORE_OPTIONS[index] {
                MoreOption::Pause(None) => {
                    let color = if index == focus {
                        theme::LAMP_AMBER_INK
                    } else {
                        theme::TEXT_PRIMARY
                    };
                    widgets::icon(icons::pause(), 14.0, color)
                }
                _ => text(label)
                    .font(if mono { theme::MONO } else { theme::UI })
                    .size(12)
                    .into(),
            };
            button(container(content).center_x(Length::Fill))
                .width(Length::Fill)
                .height(32)
                .padding(Padding::from([8.0, 4.0]))
                .style(theme::menu_option(index == focus))
                .on_press(Message::More(index))
                .into()
        };
        let until = match self.setup.day_end {
            Some(end) => format!("Until {}", format::clock(end, tz)),
            None => String::from("Until end of day"),
        };
        let keep = column![
            rich(vec![
                span("Keep logging ", theme::UI, theme::TEXT_MUTED),
                span(&name, theme::UI, theme::TEXT_PRIMARY),
                span(" without asking for", theme::UI, theme::TEXT_MUTED),
            ]),
            row![
                option(0, "30m".into(), true),
                option(1, "1h".into(), true),
                option(2, "2h".into(), true),
                option(3, until, false),
            ]
            .spacing(6),
        ]
        .spacing(6);
        let snooze = column![
            theme::label("Ask me again in"),
            row![option(4, "5m".into(), true), option(5, "10m".into(), true)].spacing(6),
        ]
        .spacing(6)
        .width(Length::Fill);
        let pause = column![
            theme::label("Pause tracking for"),
            row![
                option(6, "30m".into(), true),
                option(7, "1h".into(), true),
                option(8, String::new(), false)
            ]
            .spacing(6),
        ]
        .spacing(6)
        .width(Length::Fill);
        container(
            column![
                keep,
                container(Space::new())
                    .width(Length::Fill)
                    .height(1)
                    .style(theme::menu_divider),
                row![snooze, pause].spacing(theme::S_MD),
            ]
            .spacing(10),
        )
        .padding(10)
        .width(Length::Fill)
        .style(theme::floating_menu)
        .into()
    }
}

fn more_key() -> String {
    format!("{}K", theme::command_key())
}

fn late_key() -> String {
    format!("{}L", theme::command_key())
}

fn span<'a>(content: &str, font: Font, color: iced::Color) -> Span<'a, (), Font> {
    Span::new(content.to_owned()).font(font).color(color)
}

fn rich<'a>(spans: Vec<Span<'a, (), Font>>) -> Element<'a, Message> {
    Rich::from_iter(spans).size(12).into()
}

/// `Code review · Work`, with the matched fragments in amber.
fn name_text<'a>(activity: &ActivityName, needles: &[String]) -> Element<'a, Message> {
    let mut spans = Vec::new();
    let push = |spans: &mut Vec<Span<'a, (), Font>>, part: &str, base: iced::Color| {
        for (run, on) in format::highlight(part, needles) {
            let (font, color) = if on {
                (theme::UI_SEMIBOLD, theme::LAMP_AMBER)
            } else {
                (theme::UI, base)
            };
            spans.push(Span::new(run).font(font).color(color));
        }
    };
    push(&mut spans, &activity.name, theme::TEXT_PRIMARY);
    if let Some(category) = &activity.category {
        spans.push(Span::new(" · ").font(theme::UI).color(theme::TEXT_MUTED));
        push(&mut spans, category, theme::TEXT_MUTED);
    }
    Rich::from_iter(spans).size(14).width(Length::Fill).into()
}

fn new_hint<'a>() -> Element<'a, Message> {
    rich(vec![
        span("Write ", theme::UI, theme::TEXT_MUTED),
        span("Category: Activity", theme::MONO, theme::TEXT_PRIMARY),
        span(
            " to group it, or just a name.",
            theme::UI,
            theme::TEXT_MUTED,
        ),
    ])
}

/// The rows under the field.
///
/// - Untouched field: the previous answer first, then recent activities.
/// - After typing: activities containing the text first (Enter picks the best match),
///   then the text as a new activity unless it already exists, then activities sharing
///   a word with it. A new row is never pushed out of the list.
pub fn rows(
    input: &str,
    edited: bool,
    previous: Option<&ActivityName>,
    suggestions: &[ActivityName],
    max: usize,
) -> Vec<Row> {
    let typed = input.trim();
    if !edited || typed.is_empty() {
        let mut rows = Vec::new();
        if let Some(previous) = previous.filter(|_| !edited) {
            rows.push(Row {
                activity: previous.clone(),
                kind: RowKind::Previous,
                needles: Vec::new(),
            });
        }
        rows.extend(
            suggestions
                .iter()
                .filter(|s| Some(*s) != previous.filter(|_| !edited))
                .map(|s| Row {
                    activity: s.clone(),
                    kind: RowKind::Recent,
                    needles: Vec::new(),
                }),
        );
        rows.truncate(max);
        return rows;
    }

    let lower = typed.to_lowercase();
    let display = |a: &ActivityName| a.to_string().to_lowercase();
    let exact = suggestions.iter().any(|s| display(s) == lower);
    let mut contains: Vec<&ActivityName> = suggestions
        .iter()
        .filter(|s| display(s).contains(&lower))
        .collect();
    // An exact match goes first.
    contains.sort_by_key(|s| display(s) != lower);
    let words: Vec<String> = lower
        .split(|c: char| c.is_whitespace() || c == ':')
        .filter(|w| w.chars().count() >= 2)
        .map(str::to_owned)
        .collect();
    let mut sharing: Vec<(usize, &ActivityName)> = suggestions
        .iter()
        .filter(|s| !contains.contains(s))
        .filter_map(|s| {
            let d = display(s);
            let count = words.iter().filter(|w| d.contains(w.as_str())).count();
            (count > 0).then_some((count, s))
        })
        .collect();
    sharing.sort_by_key(|(count, _)| std::cmp::Reverse(*count));

    let new = (!exact)
        .then(|| ActivityName::parse(typed).ok())
        .flatten()
        .map(|activity| Row {
            activity,
            kind: RowKind::New,
            needles: Vec::new(),
        });
    let matched = |a: &ActivityName, needles: Vec<String>| Row {
        activity: a.clone(),
        kind: RowKind::Match,
        needles,
    };

    let mut rows: Vec<Row> = contains
        .iter()
        .map(|a| matched(a, vec![lower.clone()]))
        .collect();
    let new_at = rows.len().min(max.saturating_sub(1));
    if let Some(new) = new {
        rows.insert(new_at, new);
    }
    rows.extend(sharing.iter().map(|(_, a)| matched(a, words.clone())));
    rows.truncate(max);
    rows
}

/// Typing 1–4 into the untouched, fully selected field picks that row instead of
/// replacing the text.
pub fn digit_pick(edited: bool, value: &str, rows: usize) -> Option<usize> {
    if edited {
        return None;
    }
    let digit = value
        .parse::<usize>()
        .ok()
        .filter(|d| (1..=4).contains(d))?;
    (digit <= rows).then(|| digit - 1)
}

pub fn key_command(press: &KeyPress, more_open: bool) -> Option<Command> {
    let command = press.modifiers.command();
    match press.key.as_ref() {
        Key::Named(Named::Escape) => Some(Command::Escape),
        Key::Named(Named::Enter) if !press.captured => Some(Command::Submit),
        Key::Named(Named::ArrowUp) => Some(Command::Up),
        Key::Named(Named::ArrowDown) => Some(Command::Down),
        Key::Character(c) if command && c.eq_ignore_ascii_case("k") => Some(Command::ToggleMore),
        Key::Character(c) if command && c.eq_ignore_ascii_case("l") => Some(Command::ToggleLate),
        Key::Character(c) if command && !more_open => match c {
            "1" | "2" | "3" | "4" => c.parse::<usize>().ok().map(|d| Command::Pick(d - 1)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests;
