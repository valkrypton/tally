//! The prompt scheduler: decides when to ask and what time an answer covers.
//!
//! It never reads the clock. Every call passes `now`, which keeps it deterministic and
//! lets tests replay a whole day in microseconds.

use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};
use serde::{Deserialize, Serialize};

use crate::ActivityName;
use crate::settings::{Settings, WeeklySchedule};

/// Answers more than this long after a prompt was due count as late.
pub const LATE_GRACE: SignedDuration = SignedDuration::from_mins(2);

#[derive(Debug, Clone)]
pub struct Config {
    pub interval: SignedDuration,
    pub idle_threshold: SignedDuration,
    pub schedule: WeeklySchedule,
    pub tz: TimeZone,
}

impl Config {
    pub fn from_settings(settings: &Settings, tz: TimeZone) -> Self {
        Self {
            interval: SignedDuration::from_mins(i64::from(settings.interval_minutes)),
            idle_threshold: SignedDuration::from_mins(i64::from(settings.idle_threshold_minutes)),
            schedule: settings.schedule.clone(),
            tz,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Prompt {
    /// "What are you doing?" for the interval ending at `due_at`.
    Regular { due_at: Timestamp },
    /// "You were away": before-you-left is cursor → `left_at`, away is `left_at` → `back_at`.
    Away {
        left_at: Timestamp,
        back_at: Timestamp,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    OffSchedule,
    Tracking {
        next_prompt_at: Timestamp,
    },
    Prompting(Prompt),
    Snoozed {
        due_at: Timestamp,
        until: Timestamp,
    },
    Keeping {
        activity: ActivityName,
        until: Timestamp,
    },
    Paused {
        until: Option<Timestamp>,
    },
    Away {
        since: Timestamp,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Prompt,
    Late,
    Keep,
    Away,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Prompt => "prompt",
            Source::Late => "late",
            Source::Keep => "keep",
            Source::Away => "away",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewEntry {
    pub activity: ActivityName,
    pub start: Timestamp,
    pub end: Timestamp,
    pub source: Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepUntil {
    For(SignedDuration),
    EndOfDay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Sent every ~30 s with the OS-reported time since the last input.
    Tick {
        idle_for: SignedDuration,
    },
    Answered {
        activity: ActivityName,
        apply_late: bool,
    },
    Skipped,
    Snoozed(SignedDuration),
    KeepFor {
        activity: ActivityName,
        until: KeepUntil,
    },
    Pause(Option<SignedDuration>),
    /// Pauses until today's working hours end; tracking starts again next working day.
    EndDay,
    Resume,
    /// `None` means "away / discard" for that row.
    AwayResolved {
        before: Option<ActivityName>,
        away: Option<ActivityName>,
    },
    AskNow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    ShowPrompt(Prompt),
    HidePrompt,
    Record(NewEntry),
}

/// Everything needed to resume after a restart. Stored as JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub state: State,
    pub cursor: Timestamp,
    pub last_tick: Timestamp,
    pub last_activity: Option<ActivityName>,
}

#[derive(Debug, Clone)]
pub struct Scheduler {
    config: Config,
    state: State,
    /// Time before this instant has been dealt with: recorded, skipped or excluded.
    cursor: Timestamp,
    last_tick: Timestamp,
    last_activity: Option<ActivityName>,
}

impl Scheduler {
    pub fn new(config: Config, now: Timestamp) -> Self {
        Self {
            config,
            state: State::OffSchedule,
            cursor: now,
            last_tick: now,
            last_activity: None,
        }
    }

    /// Resumes from a snapshot. If the snapshot is stale, the next tick sees the gap
    /// and treats it as time away.
    pub fn restore(config: Config, snapshot: Snapshot) -> Self {
        Self {
            config,
            state: snapshot.state,
            cursor: snapshot.cursor,
            last_tick: snapshot.last_tick,
            last_activity: snapshot.last_activity,
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            state: self.state.clone(),
            cursor: self.cursor,
            last_tick: self.last_tick,
            last_activity: self.last_activity.clone(),
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn cursor(&self) -> Timestamp {
        self.cursor
    }

    /// The activity to pre-fill in the prompt.
    pub fn last_activity(&self) -> Option<&ActivityName> {
        self.last_activity.as_ref()
    }

    /// The prompt that should be on screen, e.g. to re-show it after a restart.
    pub fn visible_prompt(&self) -> Option<&Prompt> {
        match &self.state {
            State::Prompting(prompt) => Some(prompt),
            _ => None,
        }
    }

    /// If the regular prompt is being answered late, how much time "apply to the last
    /// X min" would cover.
    pub fn late_coverage(&self, now: Timestamp) -> Option<SignedDuration> {
        let State::Prompting(Prompt::Regular { due_at }) = self.state else {
            return None;
        };
        if now.duration_since(due_at) <= LATE_GRACE {
            return None;
        }
        let end = now.min(self.period_end(self.cursor).unwrap_or(Timestamp::MAX));
        Some(end.duration_since(self.cursor))
    }

    pub fn set_config(&mut self, config: Config, now: Timestamp) {
        self.config = config;
        if matches!(self.state, State::Tracking { .. }) {
            self.continue_tracking(now);
        }
    }

    pub fn handle(&mut self, event: Event, now: Timestamp) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            Event::Tick { idle_for } => self.tick(idle_for, now, &mut actions),
            Event::Answered {
                activity,
                apply_late,
            } => self.answer(activity, apply_late, now, &mut actions),
            Event::Skipped => self.skip(now, &mut actions),
            Event::Snoozed(duration) => self.snooze(duration, now, &mut actions),
            Event::KeepFor { activity, until } => self.keep_for(activity, until, now, &mut actions),
            Event::Pause(duration) => self.pause(duration, now, &mut actions),
            Event::EndDay => self.end_day(now, &mut actions),
            Event::Resume => self.resume(now),
            Event::AwayResolved { before, away } => {
                self.resolve_away(before, away, now, &mut actions)
            }
            Event::AskNow => self.ask_now(now, &mut actions),
        }
        actions
    }

    fn tick(&mut self, idle_for: SignedDuration, now: Timestamp, actions: &mut Vec<Action>) {
        let previous_tick = self.last_tick;
        self.last_tick = now;
        let threshold = self.config.idle_threshold;

        // Leaving: a long gap between ticks means the machine slept (or tally was not
        // running); a long OS idle time means nobody is at the keyboard.
        if self.watches_presence() {
            let left_at = if now.duration_since(previous_tick) > threshold {
                Some(previous_tick)
            } else if idle_for >= threshold {
                Some(now - idle_for)
            } else {
                None
            };
            if let Some(left_at) = left_at {
                if matches!(self.state, State::Prompting(_)) {
                    actions.push(Action::HidePrompt);
                }
                self.state = State::Away {
                    since: left_at.max(self.cursor),
                };
            }
        }

        match self.state.clone() {
            State::OffSchedule => {
                if let Some((start, _)) = self.period_at(now) {
                    self.cursor = start.max(previous_tick).max(self.cursor);
                    self.continue_tracking(now);
                }
            }
            State::Tracking { next_prompt_at } => {
                if now >= next_prompt_at {
                    self.show(
                        Prompt::Regular {
                            due_at: next_prompt_at,
                        },
                        actions,
                    );
                }
            }
            State::Prompting(_) => {}
            State::Snoozed { due_at, until } => {
                if now >= until {
                    self.show(Prompt::Regular { due_at }, actions);
                }
            }
            State::Keeping { activity, until } => {
                let end = now.min(until);
                if now >= until || end.duration_since(self.cursor) >= self.config.interval {
                    self.record(&activity, self.cursor, end, Source::Keep, actions);
                    self.cursor = end;
                }
                if now >= until {
                    self.continue_tracking(now);
                }
            }
            State::Paused { until: Some(until) } if now >= until => self.resume(now),
            State::Paused { .. } => {}
            State::Away { since } => {
                if idle_for < threshold {
                    let back_at = (now - idle_for).max(since);
                    let back_at = back_at.min(self.period_end(since).unwrap_or(back_at));
                    self.show(
                        Prompt::Away {
                            left_at: since,
                            back_at,
                        },
                        actions,
                    );
                }
            }
        }
    }

    fn answer(
        &mut self,
        activity: ActivityName,
        apply_late: bool,
        now: Timestamp,
        actions: &mut Vec<Action>,
    ) {
        let State::Prompting(Prompt::Regular { due_at }) = self.state else {
            return;
        };
        let limit = self.period_end(self.cursor).unwrap_or(Timestamp::MAX);
        let late = now.duration_since(due_at) > LATE_GRACE;
        let end = if late && apply_late { now } else { due_at }.min(limit);
        let source = if end > due_at {
            Source::Late
        } else {
            Source::Prompt
        };
        self.record(&activity, self.cursor, end, source, actions);
        self.cursor = if late && !apply_late {
            now.min(limit)
        } else {
            end
        }
        .max(self.cursor);
        self.last_activity = Some(activity);
        actions.push(Action::HidePrompt);
        self.continue_tracking(now);
    }

    fn skip(&mut self, now: Timestamp, actions: &mut Vec<Action>) {
        match self.state {
            State::Prompting(Prompt::Regular { .. }) => {
                let limit = self.period_end(self.cursor).unwrap_or(Timestamp::MAX);
                self.cursor = now.min(limit).max(self.cursor);
                actions.push(Action::HidePrompt);
                self.continue_tracking(now);
            }
            State::Prompting(Prompt::Away { .. }) => self.resolve_away(None, None, now, actions),
            _ => {}
        }
    }

    fn snooze(&mut self, duration: SignedDuration, now: Timestamp, actions: &mut Vec<Action>) {
        if let State::Prompting(Prompt::Regular { due_at }) = self.state {
            self.state = State::Snoozed {
                due_at,
                until: now + duration,
            };
            actions.push(Action::HidePrompt);
        }
    }

    fn keep_for(
        &mut self,
        activity: ActivityName,
        until: KeepUntil,
        now: Timestamp,
        actions: &mut Vec<Action>,
    ) {
        let Some((_, period_end)) = self.period_at(now) else {
            return;
        };
        match &self.state {
            State::Tracking { .. } | State::Snoozed { .. } => {
                self.record(&activity, self.cursor, now, Source::Prompt, actions)
            }
            State::Prompting(Prompt::Regular { .. }) => {
                self.record(&activity, self.cursor, now, Source::Prompt, actions);
                actions.push(Action::HidePrompt);
            }
            State::Keeping {
                activity: current, ..
            } => {
                let current = current.clone();
                self.record(&current, self.cursor, now, Source::Keep, actions);
            }
            _ => return,
        }
        let until = match until {
            KeepUntil::For(duration) => now + duration,
            KeepUntil::EndOfDay => period_end,
        };
        self.cursor = now.max(self.cursor);
        self.last_activity = Some(activity.clone());
        self.state = State::Keeping {
            activity,
            until: until.min(period_end),
        };
    }

    fn pause(
        &mut self,
        duration: Option<SignedDuration>,
        now: Timestamp,
        actions: &mut Vec<Action>,
    ) {
        match self.state.clone() {
            State::OffSchedule | State::Paused { .. } => return,
            State::Keeping { activity, .. } => {
                self.record(&activity, self.cursor, now, Source::Keep, actions);
            }
            State::Prompting(_) => actions.push(Action::HidePrompt),
            _ => {}
        }
        self.state = State::Paused {
            until: duration.map(|d| now + d),
        };
    }

    fn end_day(&mut self, now: Timestamp, actions: &mut Vec<Action>) {
        let Some(end) = self.period_end(now) else {
            return;
        };
        self.pause(None, now, actions);
        if let State::Paused { until } = &mut self.state {
            *until = Some(end);
        }
    }

    fn resume(&mut self, now: Timestamp) {
        if matches!(self.state, State::Paused { .. }) {
            self.cursor = now.max(self.cursor);
            self.continue_tracking(now);
        }
    }

    fn resolve_away(
        &mut self,
        before: Option<ActivityName>,
        away: Option<ActivityName>,
        now: Timestamp,
        actions: &mut Vec<Action>,
    ) {
        let State::Prompting(Prompt::Away { left_at, back_at }) = self.state else {
            return;
        };
        if let Some(activity) = &before {
            self.record(activity, self.cursor, left_at, Source::Prompt, actions);
        }
        if let Some(activity) = &away {
            self.record(activity, left_at, back_at, Source::Away, actions);
        }
        if let Some(latest) = away.or(before) {
            self.last_activity = Some(latest);
        }
        self.cursor = back_at.max(self.cursor);
        actions.push(Action::HidePrompt);
        self.continue_tracking(now);
    }

    fn ask_now(&mut self, now: Timestamp, actions: &mut Vec<Action>) {
        match self.state.clone() {
            State::Tracking { .. } | State::Snoozed { .. } => {
                self.show(Prompt::Regular { due_at: now }, actions)
            }
            State::Keeping { activity, .. } => {
                self.record(&activity, self.cursor, now, Source::Keep, actions);
                self.cursor = now.max(self.cursor);
                self.show(Prompt::Regular { due_at: now }, actions);
            }
            State::Prompting(prompt) => actions.push(Action::ShowPrompt(prompt)),
            _ => {}
        }
    }

    /// Sets the next state from the cursor: tracking inside a working period, otherwise off.
    fn continue_tracking(&mut self, now: Timestamp) {
        self.state = match self.period_at(now) {
            Some((start, end)) => {
                self.cursor = self.cursor.max(start);
                State::Tracking {
                    next_prompt_at: (self.cursor + self.config.interval).min(end),
                }
            }
            None => State::OffSchedule,
        };
    }

    fn show(&mut self, prompt: Prompt, actions: &mut Vec<Action>) {
        self.state = State::Prompting(prompt.clone());
        actions.push(Action::ShowPrompt(prompt));
    }

    fn record(
        &self,
        activity: &ActivityName,
        start: Timestamp,
        end: Timestamp,
        source: Source,
        actions: &mut Vec<Action>,
    ) {
        if end > start {
            actions.push(Action::Record(NewEntry {
                activity: activity.clone(),
                start,
                end,
                source,
            }));
        }
    }

    fn watches_presence(&self) -> bool {
        matches!(
            self.state,
            State::Tracking { .. }
                | State::Prompting(Prompt::Regular { .. })
                | State::Snoozed { .. }
        )
    }

    fn period_at(&self, at: Timestamp) -> Option<(Timestamp, Timestamp)> {
        self.config.schedule.period_containing(at, &self.config.tz)
    }

    fn period_end(&self, at: Timestamp) -> Option<Timestamp> {
        self.period_at(at).map(|(_, end)| end)
    }
}

#[cfg(test)]
mod tests;
