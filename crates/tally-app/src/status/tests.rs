use jiff::Timestamp;
use jiff::tz::TimeZone;
use tally_core::ActivityName;
use tally_core::scheduler::State;

use super::*;

fn at(h: i64, m: i64) -> Timestamp {
    Timestamp::from_second(h * 3600 + m * 60).unwrap()
}

#[test]
fn tracking_shows_the_next_ask() {
    let status = Status::of(
        &State::Tracking {
            next_prompt_at: at(16, 45),
        },
        &TimeZone::UTC,
    );
    assert_eq!(
        status,
        Status {
            tracking: true,
            text: "Tracking, next ask".into(),
            figure: Some("16:45".into())
        }
    );
}

#[test]
fn paused_and_off_schedule_have_no_green_dot() {
    let tz = TimeZone::UTC;
    assert!(!Status::of(&State::Paused { until: None }, &tz).tracking);
    assert_eq!(
        Status::of(
            &State::Paused {
                until: Some(at(14, 30))
            },
            &tz
        )
        .figure
        .as_deref(),
        Some("14:30")
    );
    assert!(!Status::of(&State::OffSchedule, &tz).tracking);
    assert!(!Status::of(&State::Away { since: at(9, 0) }, &tz).tracking);
}

#[test]
fn keeping_names_the_activity() {
    let state = State::Keeping {
        activity: ActivityName::parse("Work: Code review").unwrap(),
        until: at(11, 0),
    };
    let status = Status::of(&state, &TimeZone::UTC);
    assert_eq!(status.text, "Logging Code review until");
    assert_eq!(status.figure.as_deref(), Some("11:00"));
}
