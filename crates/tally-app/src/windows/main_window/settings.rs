//! Settings: interval, idle threshold, start at login, the global shortcut, the data
//! file and the weekly schedule. Changes save as they're made. Artboard B-Settings.

use iced::alignment::{Horizontal, Vertical};
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::{Space, button, checkbox, column, container, row, text, text_input};
use iced::{Element, Length, Padding};
use jiff::civil::Time;
use tally_core::settings::{Settings, WeeklySchedule, WorkHours};

use crate::platform::hotkey;
use crate::theme;
use crate::windows::KeyPress;
use crate::windows::widgets::stepper;

pub const DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

const STEP: u32 = 5;
pub const IDLE_MIN: u32 = 5;
pub const IDLE_MAX: u32 = 120;

/// Moves a minute setting by one step of 5, landing on multiples of 5.
pub fn step_minutes(current: u32, up: bool, min: u32, max: u32) -> u32 {
    let next = if up {
        (current / STEP + 1) * STEP
    } else {
        current.saturating_sub(1) / STEP * STEP
    };
    next.clamp(min, max)
}

/// `09:00`, `9:00` or `9` → 09:00. Anything else is invalid.
pub fn parse_time(input: &str) -> Option<Time> {
    let input = input.trim();
    let (hour, minute) = match input.split_once(':') {
        Some((h, m)) if m.len() == 2 => (h.parse::<i8>().ok()?, m.parse::<i8>().ok()?),
        Some(_) => return None,
        None => (input.parse::<i8>().ok()?, 0),
    };
    Time::new(hour, minute, 0, 0).ok()
}

/// One weekday row as typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayHours {
    pub enabled: bool,
    pub start: String,
    pub end: String,
}

impl DayHours {
    /// The hours, or which fields are wrong: (start bad, end bad). End must be after start.
    pub fn parse(&self) -> Result<WorkHours, (bool, bool)> {
        match (parse_time(&self.start), parse_time(&self.end)) {
            (Some(start), Some(end)) if start < end => Ok(WorkHours { start, end }),
            (Some(_), Some(_)) => Err((false, true)),
            (start, end) => Err((start.is_none(), end.is_none())),
        }
    }
}

fn day_rows(schedule: &WeeklySchedule) -> [DayHours; 7] {
    std::array::from_fn(|i| {
        let hours = schedule.days[i].unwrap_or(WorkHours {
            start: Time::constant(9, 0, 0, 0),
            end: Time::constant(18, 0, 0, 0),
        });
        DayHours {
            enabled: schedule.days[i].is_some(),
            start: hours.start.strftime("%H:%M").to_string(),
            end: hours.end.strftime("%H:%M").to_string(),
        }
    })
}

#[derive(Debug, Clone)]
pub enum Message {
    Interval(bool),
    Idle(bool),
    StartAtLogin(bool),
    RecordShortcut,
    DayEnabled(usize, bool),
    DayStart(usize, String),
    DayEnd(usize, String),
    OpenFolder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Persist and apply these settings.
    Save(Settings),
    /// Register this shortcut; on success save it (see `shortcut_result`).
    Shortcut(String),
    OpenFolder,
}

#[derive(Debug, Clone)]
pub struct SettingsView {
    settings: Settings,
    days: [DayHours; 7],
    recording: bool,
    shortcut_error: Option<String>,
    autostart_error: Option<String>,
    save_error: Option<String>,
    data_file: String,
}

impl SettingsView {
    pub fn new(settings: Settings, data_file: String) -> Self {
        Self {
            days: day_rows(&settings.schedule),
            settings,
            recording: false,
            shortcut_error: None,
            autostart_error: None,
            save_error: None,
            data_file,
        }
    }

    #[cfg(test)]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    #[cfg(test)]
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// The app tried to register a recorded shortcut.
    pub fn shortcut_result(&mut self, spec: String, result: Result<(), String>) -> Option<Outcome> {
        self.recording = false;
        match result {
            Ok(()) => {
                self.shortcut_error = None;
                self.settings.shortcut = spec;
                Some(Outcome::Save(self.settings.clone()))
            }
            Err(error) => {
                self.shortcut_error = Some(error);
                None
            }
        }
    }

    /// Shows a registration failure from startup.
    pub fn set_shortcut_error(&mut self, error: Option<String>) {
        self.shortcut_error = error;
    }

    pub fn set_autostart_error(&mut self, error: Option<String>) {
        self.autostart_error = error;
    }

    pub fn set_save_error(&mut self, error: Option<String>) {
        self.save_error = error;
    }

    fn save(&self) -> Option<Outcome> {
        Some(Outcome::Save(self.settings.clone()))
    }

    /// Re-reads row `day` and saves the schedule if that row is valid.
    fn day_changed(&mut self, day: usize) -> Option<Outcome> {
        let row = &self.days[day];
        let hours = if row.enabled {
            Some(row.parse().ok()?)
        } else {
            None
        };
        if self.settings.schedule.days[day] == hours {
            return None;
        }
        self.settings.schedule.days[day] = hours;
        self.save()
    }

    pub fn update(&mut self, message: Message) -> Option<Outcome> {
        match message {
            Message::Interval(up) => {
                self.settings.interval_minutes = step_minutes(
                    self.settings.interval_minutes,
                    up,
                    Settings::MIN_INTERVAL,
                    Settings::MAX_INTERVAL,
                );
                self.save()
            }
            Message::Idle(up) => {
                self.settings.idle_threshold_minutes =
                    step_minutes(self.settings.idle_threshold_minutes, up, IDLE_MIN, IDLE_MAX);
                self.save()
            }
            Message::StartAtLogin(on) => {
                self.settings.start_at_login = on;
                self.save()
            }
            Message::RecordShortcut => {
                self.recording = !self.recording;
                None
            }
            Message::DayEnabled(day, on) => {
                self.days[day].enabled = on;
                self.day_changed(day)
            }
            Message::DayStart(day, value) => {
                self.days[day].start = value;
                self.day_changed(day)
            }
            Message::DayEnd(day, value) => {
                self.days[day].end = value;
                self.day_changed(day)
            }
            Message::OpenFolder => Some(Outcome::OpenFolder),
        }
    }

    /// While recording, the next key press with a modifier becomes the shortcut; Esc
    /// stops recording.
    pub fn key(&mut self, press: &KeyPress) -> Option<Outcome> {
        if !self.recording {
            return None;
        }
        if press.key == Key::Named(Named::Escape) {
            self.recording = false;
            return None;
        }
        hotkey::spec_from_key(press.modifiers, &press.physical).map(Outcome::Shortcut)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let section = |title: &'static str| {
            text(title)
                .font(theme::UI_SEMIBOLD)
                .size(13)
                .color(theme::TEXT_MUTED)
        };
        let described = |name: &'static str, note: &'static str| {
            column![theme::body(name), theme::label(note)].spacing(theme::S_HAIR)
        };
        let setting_row = |left: Element<'static, Message>, right: Element<'static, Message>| {
            row![container(left).width(Length::Fill), right]
                .spacing(theme::S_MD)
                .align_y(Vertical::Center)
        };

        let interval = self.settings.interval_minutes;
        let idle = self.settings.idle_threshold_minutes;
        let asking = column![
            section("Asking"),
            setting_row(
                described("Ask every", "5 to 60 minutes").into(),
                stepper(
                    format!("{interval} min"),
                    (interval > Settings::MIN_INTERVAL).then_some(Message::Interval(false)),
                    (interval < Settings::MAX_INTERVAL).then_some(Message::Interval(true)),
                ),
            ),
            setting_row(
                described("Count me away after", "No keyboard or mouse this long").into(),
                stepper(
                    format!("{idle} min"),
                    (idle > IDLE_MIN).then_some(Message::Idle(false)),
                    (idle < IDLE_MAX).then_some(Message::Idle(true)),
                ),
            ),
        ]
        .spacing(14);

        let mut login = column![setting_row(
            theme::body("Start tally when I log in").into(),
            checkbox(self.settings.start_at_login)
                .on_toggle(Message::StartAtLogin)
                .size(16)
                .style(theme::checkbox_style)
                .into(),
        )]
        .spacing(theme::S_XS);
        if let Some(error) = &self.autostart_error {
            login = login.push(theme::label(error.clone()).color(theme::ALERT));
        }

        let caps: Element<'_, Message> = if self.recording {
            text("Press keys…")
                .size(13)
                .color(theme::LAMP_AMBER_INK)
                .into()
        } else {
            row(
                hotkey::caps(&self.settings.shortcut, cfg!(target_os = "macos"))
                    .into_iter()
                    .map(|cap| {
                        container(text(cap).font(theme::MONO).size(13))
                            .padding(Padding::from([0.0, 5.0]))
                            .style(theme::shortcut_cap)
                            .into()
                    }),
            )
            .spacing(theme::S_XS)
            .into()
        };
        let mut shortcut = column![setting_row(
            described("Ask now shortcut", "Works from any app").into(),
            button(caps)
                .height(32)
                .padding(Padding::from([6.0, 10.0]))
                .style(theme::shortcut_button(self.recording))
                .on_press(Message::RecordShortcut)
                .into(),
        )]
        .spacing(theme::S_XS);
        if let Some(error) = &self.shortcut_error {
            shortcut = shortcut.push(theme::label(error.clone()).color(theme::ALERT));
        }

        let data = column![
            theme::body("Data file"),
            row![
                text(self.data_file.clone())
                    .font(theme::MONO)
                    .size(12)
                    .color(theme::TEXT_MUTED)
                    .width(Length::Fill),
                button(theme::small("Open folder"))
                    .height(30)
                    .padding(Padding::from([7.0, 10.0]))
                    .style(theme::button_outlined)
                    .on_press(Message::OpenFolder),
            ]
            .spacing(theme::S_SM)
            .align_y(Vertical::Center),
        ]
        .spacing(6);

        let system = column![section("System"), login, shortcut, data].spacing(14);

        let mut hours = column![
            column![
                section("Working hours"),
                theme::label("tally only asks inside these hours."),
            ]
            .spacing(theme::S_HAIR)
        ]
        .spacing(10);
        for (index, day) in self.days.iter().enumerate() {
            hours = hours.push(self.day_view(index, day));
        }

        let mut page = column![
            theme::headline("Settings"),
            row![
                column![asking, system]
                    .spacing(theme::S_XXL)
                    .width(Length::Fill),
                hours.width(Length::Fill),
            ]
            .spacing(32),
            Space::new().height(Length::Fill),
        ]
        .spacing(theme::S_XL)
        .height(Length::Fill);
        if let Some(error) = &self.save_error {
            page = page
                .push(theme::label(format!("Couldn't save settings: {error}")).color(theme::ALERT));
        }
        page.push(theme::label("Changes save as you make them."))
            .into()
    }

    fn day_view<'a>(&'a self, index: usize, day: &'a DayHours) -> Element<'a, Message> {
        let (bad_start, bad_end) = if day.enabled {
            day.parse().err().unwrap_or((false, false))
        } else {
            (false, false)
        };
        let time = |value: &'a str, bad: bool, on_input: fn(usize, String) -> Message| {
            let mut input = text_input("09:00", value)
                .font(theme::MONO)
                .size(13)
                .align_x(Horizontal::Center)
                .padding(Padding::from([8.0, 6.0]))
                .width(Length::Fill)
                .style(theme::field(if bad {
                    theme::FieldLook::Error
                } else {
                    theme::FieldLook::Resting
                }));
            if day.enabled {
                input = input.on_input(move |v| on_input(index, v));
            }
            input
        };
        let dash_color = if day.enabled {
            theme::TEXT_MUTED
        } else {
            theme::TEXT_DISABLED
        };
        row![
            checkbox(day.enabled)
                .label(DAYS[index])
                .on_toggle(move |on| Message::DayEnabled(index, on))
                .size(16)
                .text_size(14)
                .font(theme::UI)
                .style(theme::checkbox_style)
                .width(96),
            time(&day.start, bad_start, Message::DayStart),
            text("–")
                .color(dash_color)
                .width(16)
                .align_x(Horizontal::Center),
            time(&day.end, bad_end, Message::DayEnd),
        ]
        .spacing(theme::S_SM)
        .height(36)
        .align_y(Vertical::Center)
        .into()
    }
}

#[cfg(test)]
mod tests;
