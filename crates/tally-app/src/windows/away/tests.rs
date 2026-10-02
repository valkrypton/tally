use iced::keyboard::Modifiers;

use super::*;

fn act(name: &str) -> ActivityName {
    ActivityName::parse(name).unwrap()
}

fn at(minute: i64) -> Timestamp {
    Timestamp::from_second(9 * 3600 + minute * 60).unwrap()
}

fn setup(cursor: i64) -> Setup {
    Setup {
        cursor: at(cursor),
        left_at: at(40),
        back_at: at(85),
        previous: Some(act("Work: Code review")),
        suggestions: vec![
            act("Work: Meetings"),
            act("Lunch"),
            act("Call"),
            act("Email"),
        ],
        tz: TimeZone::UTC,
    }
}

#[test]
fn before_is_prefilled_and_away_starts_uncounted() {
    let away = Away::new(setup(25));
    assert!(away.has_before());
    assert_eq!(away.answers(), (Some(act("Work: Code review")), None));
    assert_eq!(away.logged_seconds(), 15 * 60);
}

#[test]
fn chips_fill_the_away_field() {
    let mut away = Away::new(setup(25));
    away.update(Message::Chip(Some(0)));
    assert_eq!(away.answers().1, Some(act("Work: Meetings")));
    assert_eq!(away.logged_seconds(), 60 * 60);
    away.update(Message::Chip(None));
    assert_eq!(away.answers().1, None);
}

#[test]
fn submit_resolves_both_rows() {
    let mut away = Away::new(setup(25));
    away.update(Message::During("Lunch".into()));
    assert_eq!(
        away.update(Message::Submit),
        Some(Outcome::Resolve {
            before: Some(act("Work: Code review")),
            away: Some(act("Lunch"))
        })
    );
}

#[test]
fn escape_discards_both() {
    let mut away = Away::new(setup(25));
    let esc = KeyPress::new(Key::Named(Named::Escape), Modifiers::empty());
    assert_eq!(
        away.key(&esc),
        Some(Outcome::Resolve {
            before: None,
            away: None
        })
    );
}

#[test]
fn command_digits_pick_chips() {
    let mut away = Away::new(setup(25));
    away.key(&KeyPress::new(
        Key::Character("2".into()),
        Modifiers::COMMAND,
    ));
    assert_eq!(away.answers().1, Some(act("Lunch")));
    away.key(&KeyPress::new(
        Key::Character("4".into()),
        Modifiers::COMMAND,
    ));
    assert_eq!(away.answers().1, None);
}

#[test]
fn no_before_row_when_the_cursor_is_already_at_leaving_time() {
    let away = Away::new(setup(40));
    assert!(!away.has_before());
    assert_eq!(away.first_field(), away_id());
    assert_eq!(away.answers().0, None);
}

#[test]
fn clearing_before_leaves_it_uncounted() {
    let mut away = Away::new(setup(25));
    away.update(Message::Before("  ".into()));
    assert_eq!(away.answers().0, None);
    assert_eq!(away.logged_seconds(), 0);
}
