use jiff::civil::{Time, date};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use super::*;
use crate::settings::{WeeklySchedule, WorkHours};

/// 2026-10-05 (a Monday) at `h:m` UTC.
fn t(h: i8, m: i8) -> Timestamp {
    date(2026, 10, 5)
        .at(h, m, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp()
}

fn mins(m: i64) -> SignedDuration {
    SignedDuration::from_mins(m)
}

fn act(name: &str) -> ActivityName {
    ActivityName::parse(name).unwrap()
}

/// 15-minute interval, 10-minute idle threshold, working hours 09:00–18:00 every day.
fn config() -> Config {
    let hours = WorkHours {
        start: Time::constant(9, 0, 0, 0),
        end: Time::constant(18, 0, 0, 0),
    };
    Config {
        interval: mins(15),
        idle_threshold: mins(10),
        schedule: WeeklySchedule {
            days: [Some(hours); 7],
        },
        tz: TimeZone::UTC,
    }
}

/// Drives a scheduler with ticks every 30 s, like the app does.
struct Harness {
    s: Scheduler,
    now: Timestamp,
}

impl Harness {
    /// Starts at 09:00 with tracking already running.
    fn at_nine() -> Self {
        let mut h = Self {
            s: Scheduler::new(config(), t(8, 59)),
            now: t(8, 59),
        };
        h.tick(t(9, 0), 0);
        assert_eq!(
            h.s.state(),
            &State::Tracking {
                next_prompt_at: t(9, 15)
            }
        );
        h
    }

    /// Ticks every 30 s up to and including `until`, with `idle` minutes of OS idle time
    /// at every tick. Returns all actions produced.
    fn run_until(&mut self, until: Timestamp, idle: i64) -> Vec<Action> {
        let mut actions = Vec::new();
        while self.now < until {
            let next = (self.now + SignedDuration::from_secs(30)).min(until);
            actions.extend(self.tick(next, idle));
        }
        actions
    }

    fn tick(&mut self, at: Timestamp, idle: i64) -> Vec<Action> {
        self.now = at;
        self.s.handle(
            Event::Tick {
                idle_for: mins(idle),
            },
            at,
        )
    }

    fn send(&mut self, event: Event, at: Timestamp) -> Vec<Action> {
        self.now = at;
        self.s.handle(event, at)
    }
}

fn records(actions: &[Action]) -> Vec<(String, Timestamp, Timestamp, Source)> {
    actions
        .iter()
        .filter_map(|a| match a {
            Action::Record(e) => Some((e.activity.to_string(), e.start, e.end, e.source)),
            _ => None,
        })
        .collect()
}

fn answered(activity: &str, apply_late: bool) -> Event {
    Event::Answered {
        activity: act(activity),
        apply_late,
    }
}

#[test]
fn starts_tracking_at_schedule_start() {
    let mut s = Scheduler::new(config(), t(8, 50));
    assert!(
        s.handle(Event::Tick { idle_for: mins(0) }, t(8, 59))
            .is_empty()
    );
    assert_eq!(s.state(), &State::OffSchedule);
    s.handle(Event::Tick { idle_for: mins(0) }, t(9, 0));
    assert_eq!(s.cursor(), t(9, 0));
    assert_eq!(
        s.state(),
        &State::Tracking {
            next_prompt_at: t(9, 15)
        }
    );
}

#[test]
fn launching_mid_day_starts_the_cursor_at_launch() {
    let mut s = Scheduler::new(config(), t(11, 7));
    s.handle(Event::Tick { idle_for: mins(0) }, t(11, 7));
    assert_eq!(s.cursor(), t(11, 7));
    assert_eq!(
        s.state(),
        &State::Tracking {
            next_prompt_at: t(11, 22)
        }
    );
}

#[test]
fn prompts_after_interval_and_records_answer() {
    let mut h = Harness::at_nine();
    let actions = h.run_until(t(9, 15), 0);
    assert_eq!(
        actions,
        vec![Action::ShowPrompt(Prompt::Regular { due_at: t(9, 15) })]
    );

    let actions = h.send(answered("Work: Meeting", false), t(9, 16));
    assert_eq!(
        records(&actions),
        vec![("Work: Meeting".into(), t(9, 0), t(9, 15), Source::Prompt)]
    );
    assert!(actions.contains(&Action::HidePrompt));
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(9, 30)
        }
    );
    assert_eq!(h.s.last_activity(), Some(&act("Work: Meeting")));
}

#[test]
fn late_answer_with_option_ticked_covers_until_now() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    assert_eq!(h.s.late_coverage(t(9, 16)), None);
    assert_eq!(h.s.late_coverage(t(9, 37)), Some(mins(37)));

    let actions = h.send(answered("Writing", true), t(9, 37));
    assert_eq!(
        records(&actions),
        vec![("Writing".into(), t(9, 0), t(9, 37), Source::Late)]
    );
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(9, 52)
        }
    );
}

#[test]
fn late_answer_with_option_unticked_leaves_gap() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    let actions = h.send(answered("Writing", false), t(9, 37));
    assert_eq!(
        records(&actions),
        vec![("Writing".into(), t(9, 0), t(9, 15), Source::Prompt)]
    );
    assert_eq!(h.s.cursor(), t(9, 37));
}

#[test]
fn skip_leaves_gap() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    let actions = h.send(Event::Skipped, t(9, 16));
    assert_eq!(actions, vec![Action::HidePrompt]);
    assert_eq!(h.s.cursor(), t(9, 16));
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(9, 31)
        }
    );
}

#[test]
fn snooze_brings_the_prompt_back_then_late_rule_applies() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    let actions = h.send(Event::Snoozed(mins(10)), t(9, 15));
    assert_eq!(actions, vec![Action::HidePrompt]);

    let actions = h.run_until(t(9, 25), 0);
    assert_eq!(
        actions,
        vec![Action::ShowPrompt(Prompt::Regular { due_at: t(9, 15) })]
    );
    let actions = h.send(answered("Email", true), t(9, 25));
    assert_eq!(
        records(&actions),
        vec![("Email".into(), t(9, 0), t(9, 25), Source::Late)]
    );
}

#[test]
fn keep_for_records_silently_in_chunks_then_resumes_prompting() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    let keep = Event::KeepFor {
        activity: act("Work: Deep work"),
        until: KeepUntil::For(mins(60)),
    };
    let actions = h.send(keep, t(9, 15));
    assert_eq!(
        records(&actions),
        vec![("Work: Deep work".into(), t(9, 0), t(9, 15), Source::Prompt)]
    );

    // No prompts while keeping, even with no input at all.
    let actions = h.run_until(t(10, 15), 30);
    assert!(!actions.iter().any(|a| matches!(a, Action::ShowPrompt(_))));
    let recorded = records(&actions);
    assert_eq!(recorded.first().unwrap().1, t(9, 15));
    assert_eq!(recorded.last().unwrap().2, t(10, 15));
    assert!(recorded.iter().all(|r| r.3 == Source::Keep));
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(10, 30)
        }
    );
}

#[test]
fn ask_now_ends_keep_early() {
    let mut h = Harness::at_nine();
    h.send(
        Event::KeepFor {
            activity: act("Call"),
            until: KeepUntil::EndOfDay,
        },
        t(9, 5),
    );
    assert_eq!(
        h.s.state(),
        &State::Keeping {
            activity: act("Call"),
            until: t(18, 0)
        }
    );
    h.run_until(t(9, 12), 0);
    let actions = h.send(Event::AskNow, t(9, 12));
    assert_eq!(
        records(&actions),
        vec![("Call".into(), t(9, 5), t(9, 12), Source::Keep)]
    );
    assert!(actions.contains(&Action::ShowPrompt(Prompt::Regular { due_at: t(9, 12) })));
}

#[test]
fn pause_and_resume_drop_the_paused_time() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 10), 0);
    h.send(Event::Pause(None), t(9, 10));
    assert!(h.run_until(t(11, 0), 0).is_empty());
    h.send(Event::Resume, t(11, 0));
    assert_eq!(h.s.cursor(), t(11, 0));
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(11, 15)
        }
    );
}

#[test]
fn pause_for_a_duration_resumes_automatically() {
    let mut h = Harness::at_nine();
    h.send(Event::Pause(Some(mins(30))), t(9, 5));
    h.run_until(t(9, 35), 0);
    assert_eq!(h.s.cursor(), t(9, 35));
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(9, 50)
        }
    );
}

#[test]
fn short_idle_counts_as_normal_time() {
    let mut h = Harness::at_nine();
    let actions = h.run_until(t(9, 15), 9);
    assert_eq!(
        actions,
        vec![Action::ShowPrompt(Prompt::Regular { due_at: t(9, 15) })]
    );
}

#[test]
fn long_idle_asks_about_both_rows_on_return() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 10), 0);
    // Idle from 09:10; at 09:20 the idle time reaches the threshold.
    let mut actions = Vec::new();
    while h.now < t(9, 55) {
        let next = h.now + SignedDuration::from_secs(30);
        let idle = next.duration_since(t(9, 10)).as_mins();
        actions.extend(h.tick(next, idle));
    }
    // The 09:15 prompt appears (idle is still under the threshold), then is hidden at
    // 09:20 when the idle time reaches it. Nothing else is shown while away.
    assert_eq!(
        actions,
        vec![
            Action::ShowPrompt(Prompt::Regular { due_at: t(9, 15) }),
            Action::HidePrompt
        ]
    );
    assert_eq!(h.s.state(), &State::Away { since: t(9, 10) });

    let actions = h.tick(t(9, 56), 0);
    let away = Prompt::Away {
        left_at: t(9, 10),
        back_at: t(9, 56),
    };
    assert_eq!(actions, vec![Action::ShowPrompt(away)]);

    let resolve = Event::AwayResolved {
        before: Some(act("Work: Code")),
        away: Some(act("Work: Meeting")),
    };
    let actions = h.send(resolve, t(9, 57));
    assert_eq!(
        records(&actions),
        vec![
            ("Work: Code".into(), t(9, 0), t(9, 10), Source::Prompt),
            ("Work: Meeting".into(), t(9, 10), t(9, 56), Source::Away),
        ]
    );
    assert_eq!(h.s.last_activity(), Some(&act("Work: Meeting")));
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(10, 11)
        }
    );
}

#[test]
fn idle_while_prompt_is_open_hides_it_and_folds_into_away() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    let actions = h.tick(t(9, 26), 11);
    assert_eq!(actions, vec![Action::HidePrompt]);
    assert_eq!(h.s.state(), &State::Away { since: t(9, 15) });
}

#[test]
fn discarding_away_time_records_nothing() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 5), 0);
    h.tick(t(9, 25), 20);
    h.tick(t(9, 40), 0);
    let actions = h.send(
        Event::AwayResolved {
            before: None,
            away: None,
        },
        t(9, 40),
    );
    assert!(records(&actions).is_empty());
    assert_eq!(h.s.cursor(), t(9, 40));
}

#[test]
fn sleep_gap_between_ticks_counts_as_away() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 10), 0);
    // Machine sleeps; next tick arrives at 10:00 and the user is active.
    let actions = h.tick(t(10, 0), 0);
    assert_eq!(
        actions,
        vec![Action::ShowPrompt(Prompt::Away {
            left_at: t(9, 10),
            back_at: t(10, 0)
        })]
    );
}

#[test]
fn end_of_schedule_gets_a_final_prompt_then_turns_off() {
    let mut h = Harness {
        s: Scheduler::restore(
            config(),
            Snapshot {
                state: State::Tracking {
                    next_prompt_at: t(18, 0),
                },
                cursor: t(17, 50),
                last_tick: t(17, 50),
                last_activity: None,
            },
        ),
        now: t(17, 50),
    };
    let actions = h.run_until(t(18, 0), 0);
    assert_eq!(
        actions,
        vec![Action::ShowPrompt(Prompt::Regular { due_at: t(18, 0) })]
    );

    // Even a late answer with the option ticked stops at the end of the day.
    let actions = h.send(answered("Admin", true), t(18, 20));
    assert_eq!(
        records(&actions),
        vec![("Admin".into(), t(17, 50), t(18, 0), Source::Prompt)]
    );
    assert_eq!(h.s.state(), &State::OffSchedule);
    assert_eq!(h.s.late_coverage(t(18, 20)), None);
}

#[test]
fn away_overnight_is_cut_at_the_end_of_the_working_day() {
    let mut h = Harness {
        s: Scheduler::restore(
            config(),
            Snapshot {
                state: State::Tracking {
                    next_prompt_at: t(17, 45),
                },
                cursor: t(17, 30),
                last_tick: t(17, 40),
                last_activity: None,
            },
        ),
        now: t(17, 40),
    };
    let next_morning = date(2026, 10, 6)
        .at(9, 5, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp();
    let actions = h.tick(next_morning, 0);
    assert_eq!(
        actions,
        vec![Action::ShowPrompt(Prompt::Away {
            left_at: t(17, 40),
            back_at: t(18, 0)
        })]
    );
    h.send(
        Event::AwayResolved {
            before: None,
            away: None,
        },
        next_morning,
    );
    let nine = date(2026, 10, 6)
        .at(9, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp();
    assert_eq!(h.s.cursor(), nine);
}

#[test]
fn restart_after_crash_treats_downtime_as_away() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 8), 0);
    let saved = serde_json::to_string(&h.s.snapshot()).unwrap();

    let snapshot: Snapshot = serde_json::from_str(&saved).unwrap();
    let mut s = Scheduler::restore(config(), snapshot);
    let actions = s.handle(Event::Tick { idle_for: mins(0) }, t(9, 40));
    assert_eq!(
        actions,
        vec![Action::ShowPrompt(Prompt::Away {
            left_at: t(9, 8),
            back_at: t(9, 40)
        })]
    );
}

#[test]
fn restored_open_prompt_is_reported_as_visible() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    let s = Scheduler::restore(config(), h.s.snapshot());
    assert_eq!(
        s.visible_prompt(),
        Some(&Prompt::Regular { due_at: t(9, 15) })
    );
}

#[test]
fn events_that_do_not_apply_are_ignored() {
    let mut h = Harness::at_nine();
    assert!(h.send(answered("X", false), t(9, 1)).is_empty());
    assert!(h.send(Event::Snoozed(mins(5)), t(9, 1)).is_empty());
    assert!(h.send(Event::Resume, t(9, 1)).is_empty());
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(9, 15)
        }
    );

    let mut off = Scheduler::new(config(), t(7, 0));
    off.handle(Event::Tick { idle_for: mins(0) }, t(7, 0));
    assert!(off.handle(Event::AskNow, t(7, 0)).is_empty());
    assert!(off.handle(Event::Pause(None), t(7, 0)).is_empty());
    assert_eq!(off.state(), &State::OffSchedule);
}

#[test]
fn changing_the_interval_reschedules_the_next_prompt() {
    let mut h = Harness::at_nine();
    let mut faster = config();
    faster.interval = mins(5);
    h.s.set_config(faster, t(9, 1));
    assert_eq!(
        h.s.state(),
        &State::Tracking {
            next_prompt_at: t(9, 5)
        }
    );
}

#[test]
fn clock_jumping_backwards_never_records_negative_time() {
    let mut h = Harness::at_nine();
    h.run_until(t(9, 15), 0);
    // The system clock is set back 20 minutes while the prompt is open.
    let actions = h.send(answered("Work: Code", true), t(8, 55));
    assert_eq!(
        records(&actions),
        vec![("Work: Code".into(), t(9, 0), t(9, 15), Source::Prompt)]
    );
    let actions = h.tick(t(8, 56), 0);
    assert!(records(&actions).is_empty());
    assert!(h.s.cursor() >= t(9, 15), "the cursor never moves backwards");
}

#[test]
fn ending_the_day_pauses_until_working_hours_end() {
    let mut h = Harness::at_nine();
    h.send(Event::Pause(None), t(9, 5));
    h.send(Event::EndDay, t(9, 10));
    assert_eq!(
        h.s.state(),
        &State::Paused {
            until: Some(t(18, 0))
        }
    );
    assert!(h.run_until(t(18, 0), 0).is_empty());
    assert_eq!(h.s.state(), &State::OffSchedule);
}
