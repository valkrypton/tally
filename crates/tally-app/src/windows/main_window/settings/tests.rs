use iced::keyboard::Modifiers;
use iced::keyboard::key::{Code, Physical};

use super::*;

fn view() -> SettingsView {
    SettingsView::new(Settings::default(), "~/tally.db".into())
}

fn saved(outcome: Option<Outcome>) -> Settings {
    match outcome {
        Some(Outcome::Save(settings)) => settings,
        other => panic!("expected a save, got {other:?}"),
    }
}

#[test]
fn minutes_step_by_five_within_limits() {
    assert_eq!(step_minutes(15, true, 5, 60), 20);
    assert_eq!(step_minutes(15, false, 5, 60), 10);
    assert_eq!(step_minutes(12, true, 5, 60), 15);
    assert_eq!(step_minutes(12, false, 5, 60), 10);
    assert_eq!(step_minutes(60, true, 5, 60), 60);
    assert_eq!(step_minutes(5, false, 5, 60), 5);
}

#[test]
fn times_parse_leniently_but_strictly_enough() {
    assert_eq!(parse_time("09:00"), Some(Time::constant(9, 0, 0, 0)));
    assert_eq!(parse_time(" 9:30 "), Some(Time::constant(9, 30, 0, 0)));
    assert_eq!(parse_time("17"), Some(Time::constant(17, 0, 0, 0)));
    assert_eq!(parse_time("24:00"), None);
    assert_eq!(parse_time("9:5"), None);
    assert_eq!(parse_time("nine"), None);
    assert_eq!(parse_time(""), None);
}

#[test]
fn interval_changes_save_immediately() {
    let mut view = view();
    assert_eq!(
        saved(view.update(Message::Interval(true))).interval_minutes,
        20
    );
    assert_eq!(
        saved(view.update(Message::Idle(false))).idle_threshold_minutes,
        5
    );
}

#[test]
fn editing_hours_saves_only_valid_rows() {
    let mut view = view();
    assert_eq!(
        view.update(Message::DayStart(0, "1".into())),
        Some(Outcome::Save({
            let mut s = Settings::default();
            s.schedule.days[0] = Some(WorkHours {
                start: Time::constant(1, 0, 0, 0),
                end: Time::constant(18, 0, 0, 0),
            });
            s
        }))
    );
    assert_eq!(
        view.update(Message::DayStart(0, "1x".into())),
        None,
        "invalid: not saved"
    );
    assert_eq!(view.days[0].parse(), Err((true, false)));
    assert_eq!(
        view.update(Message::DayStart(0, "19:00".into())),
        None,
        "start after end"
    );
    assert_eq!(view.days[0].parse(), Err((false, true)));
    assert_eq!(
        view.settings().schedule.days[0].unwrap().start,
        Time::constant(1, 0, 0, 0)
    );
}

#[test]
fn turning_a_day_on_and_off() {
    let mut view = view();
    let on = saved(view.update(Message::DayEnabled(5, true)));
    assert_eq!(
        on.schedule.days[5],
        Some(WorkHours {
            start: Time::constant(9, 0, 0, 0),
            end: Time::constant(18, 0, 0, 0)
        })
    );
    let off = saved(view.update(Message::DayEnabled(5, false)));
    assert_eq!(off.schedule.days[5], None);
    assert_eq!(
        view.days[5].start, "09:00",
        "typed hours survive switching off"
    );
}

#[test]
fn recording_a_shortcut() {
    let mut view = view();
    let press = KeyPress {
        physical: Physical::Code(Code::KeyY),
        ..KeyPress::new(Key::Character("y".into()), Modifiers::CTRL | Modifiers::ALT)
    };
    assert_eq!(view.key(&press), None, "not recording yet");
    view.update(Message::RecordShortcut);
    assert!(view.is_recording());
    assert_eq!(
        view.key(&press),
        Some(Outcome::Shortcut("Ctrl+Alt+KeyY".into()))
    );
    let saved = saved(view.shortcut_result("Ctrl+Alt+KeyY".into(), Ok(())));
    assert_eq!(saved.shortcut, "Ctrl+Alt+KeyY");
    assert!(!view.is_recording());
}

#[test]
fn a_taken_shortcut_shows_the_error_and_keeps_the_old_one() {
    let mut view = view();
    view.update(Message::RecordShortcut);
    assert_eq!(
        view.shortcut_result("Ctrl+Alt+KeyY".into(), Err(hotkey::IN_USE.into())),
        None
    );
    assert_eq!(view.shortcut_error.as_deref(), Some(hotkey::IN_USE));
    assert_eq!(view.settings().shortcut, Settings::default().shortcut);
}

#[test]
fn escape_stops_recording() {
    let mut view = view();
    view.update(Message::RecordShortcut);
    view.key(&KeyPress::new(
        Key::Named(Named::Escape),
        Modifiers::empty(),
    ));
    assert!(!view.is_recording());
}
