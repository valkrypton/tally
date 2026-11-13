//! The one-line status shown in the dashboard footer and the main-window sidebar.

use jiff::Timestamp;
use jiff::tz::TimeZone;
use tally_core::scheduler::State;

use crate::format;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Status {
    /// Draws the green dot (the only use of Tracking Green).
    pub tracking: bool,
    pub text: String,
    /// A time drawn in figures after the text.
    pub figure: Option<String>,
}

impl Status {
    pub fn of(state: &State, tz: &TimeZone) -> Self {
        let at = |text: &str, when: Timestamp, tracking| Status {
            tracking,
            text: text.to_owned(),
            figure: Some(format::clock(when, tz)),
        };
        let plain = |text: &str, tracking| Status {
            tracking,
            text: text.to_owned(),
            figure: None,
        };
        match state {
            State::Tracking { next_prompt_at } => at("Tracking, next ask", *next_prompt_at, true),
            State::Prompting(_) => plain("Tracking, asking now", true),
            State::Snoozed { until, .. } => at("Snoozed until", *until, true),
            State::Keeping { activity, until } => {
                at(&format!("Logging {} until", activity.name), *until, true)
            }
            State::Paused { until: Some(until) } => at("Paused until", *until, false),
            State::Paused { until: None } => plain("Paused", false),
            State::Away { .. } => plain("Away", false),
            State::OffSchedule => plain("Outside working hours", false),
        }
    }

    pub fn is_paused(state: &State) -> bool {
        matches!(state, State::Paused { .. })
    }
}

#[cfg(test)]
mod tests;
