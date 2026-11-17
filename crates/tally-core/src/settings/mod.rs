//! User settings and the weekly working schedule.

use jiff::Timestamp;
use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

/// Working hours for one weekday. `start` must be before `end` (same day).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkHours {
    pub start: Time,
    pub end: Time,
}

/// Working hours per weekday, Monday first. `None` means no tracking that day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeeklySchedule {
    pub days: [Option<WorkHours>; 7],
}

impl Default for WeeklySchedule {
    /// Monday to Friday, 09:00 to 18:00.
    fn default() -> Self {
        let hours = WorkHours {
            start: Time::constant(9, 0, 0, 0),
            end: Time::constant(18, 0, 0, 0),
        };
        Self {
            days: [
                Some(hours),
                Some(hours),
                Some(hours),
                Some(hours),
                Some(hours),
                None,
                None,
            ],
        }
    }
}

impl WeeklySchedule {
    /// The working period `[start, end)` that contains `at`, if any.
    pub fn period_containing(
        &self,
        at: Timestamp,
        tz: &TimeZone,
    ) -> Option<(Timestamp, Timestamp)> {
        let date = at.to_zoned(tz.clone()).date();
        let (start, end) = self.period_on(date, tz)?;
        (start <= at && at < end).then_some((start, end))
    }

    /// The working period on `date`, if that weekday has working hours.
    pub fn period_on(&self, date: Date, tz: &TimeZone) -> Option<(Timestamp, Timestamp)> {
        let offset = date.weekday().to_monday_zero_offset() as usize;
        let hours = self.days[offset]?;
        if hours.start >= hours.end {
            return None;
        }
        let start = date
            .to_datetime(hours.start)
            .to_zoned(tz.clone())
            .ok()?
            .timestamp();
        let end = date
            .to_datetime(hours.end)
            .to_zoned(tz.clone())
            .ok()?
            .timestamp();
        Some((start, end))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Minutes between prompts, 5 to 60.
    pub interval_minutes: u32,
    pub schedule: WeeklySchedule,
    /// Minutes without input before tally treats you as away.
    pub idle_threshold_minutes: u32,
    pub start_at_login: bool,
    /// Global shortcut that opens the prompt, in `global-hotkey` syntax.
    pub shortcut: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_minutes: 15,
            schedule: WeeklySchedule::default(),
            idle_threshold_minutes: 10,
            start_at_login: false,
            shortcut: String::from("Alt+Shift+KeyT"),
        }
    }
}

impl Settings {
    pub const MIN_INTERVAL: u32 = 5;
    pub const MAX_INTERVAL: u32 = 60;

    /// Clamps values into their allowed ranges.
    pub fn normalized(mut self) -> Self {
        self.interval_minutes = self
            .interval_minutes
            .clamp(Self::MIN_INTERVAL, Self::MAX_INTERVAL);
        self.idle_threshold_minutes = self.idle_threshold_minutes.clamp(1, 240);
        self
    }
}

#[cfg(test)]
mod tests;
