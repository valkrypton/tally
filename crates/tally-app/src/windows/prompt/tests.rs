use iced::keyboard::Modifiers;
use iced::keyboard::key::Named;

use super::*;

fn act(name: &str) -> ActivityName {
    ActivityName::parse(name).unwrap()
}

fn at(minute: i64) -> Timestamp {
    Timestamp::from_second(9 * 3600 + minute * 60).unwrap()
}

fn suggestions() -> Vec<ActivityName> {
    [
        "Work: Code review",
        "Work: Meetings",
        "Study: Rust book",
        "Lunch",
        "Work: Email",
    ]
    .into_iter()
    .map(act)
    .collect()
}

fn setup(late: Option<i64>) -> Setup {
    Setup {
        cursor: at(0),
        due_at: at(15),
        late: late.map(SignedDuration::from_mins),
        previous: Some(act("Work: Code review")),
        suggestions: suggestions(),
        day_end: Some(at(9 * 60)),
        tz: TimeZone::UTC,
    }
}

fn names(rows: &[Row]) -> Vec<String> {
    rows.iter().map(|r| r.activity.to_string()).collect()
}

fn press(key: Key, modifiers: Modifiers) -> KeyPress {
    KeyPress::new(key, modifiers)
}

fn ch(c: &str) -> Key {
    Key::Character(c.into())
}

#[test]
fn untouched_field_lists_previous_then_recent() {
    let rows = rows(
        "Work: Code review",
        false,
        Some(&act("Work: Code review")),
        &suggestions(),
        4,
    );
    assert_eq!(
        names(&rows),
        [
            "Work: Code review",
            "Work: Meetings",
            "Study: Rust book",
            "Lunch"
        ]
    );
    assert_eq!(rows[0].kind, RowKind::Previous);
    assert_eq!(rows[1].kind, RowKind::Recent);
}

#[test]
fn typing_puts_containing_matches_first_then_new() {
    let rows = rows("code", true, None, &suggestions(), 4);
    assert_eq!(names(&rows)[0], "Work: Code review");
    assert_eq!(rows[0].kind, RowKind::Match);
    assert_eq!(rows[0].needles, ["code"]);
    assert_eq!(rows[1].kind, RowKind::New);
    assert_eq!(rows[1].activity, act("code"));
}

#[test]
fn a_new_name_comes_first_with_word_matches_after() {
    let rows = rows("Study: Rust async", true, None, &suggestions(), 4);
    assert_eq!(rows[0].kind, RowKind::New);
    assert_eq!(rows[0].activity, act("Study: Rust async"));
    assert_eq!(names(&rows[1..]), ["Study: Rust book"]);
    assert_eq!(rows[1].needles, ["study", "rust", "async"]);
}

#[test]
fn exact_names_are_not_offered_as_new() {
    let rows = rows("work: meetings", true, None, &suggestions(), 4);
    assert_eq!(names(&rows)[0], "Work: Meetings");
    assert_eq!(rows[0].kind, RowKind::Match);
    assert!(rows.iter().all(|r| r.kind != RowKind::New));
}

#[test]
fn the_new_row_is_never_pushed_out() {
    let rows = rows("o", true, None, &suggestions(), 3);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[2].kind, RowKind::New);
}

#[test]
fn cleared_field_shows_recent_without_previous() {
    let rows = rows(
        "  ",
        true,
        Some(&act("Work: Code review")),
        &suggestions(),
        4,
    );
    assert_eq!(rows[0].kind, RowKind::Recent);
}

#[test]
fn enter_logs_the_previous_answer() {
    let mut prompt = Prompt::new(setup(None));
    assert_eq!(
        prompt.update(Message::Submit),
        Some(Outcome::Answer {
            activity: act("Work: Code review"),
            apply_late: true
        })
    );
}

#[test]
fn digits_pick_rows_only_while_the_field_is_untouched() {
    assert_eq!(digit_pick(false, "2", 4), Some(1));
    assert_eq!(digit_pick(false, "5", 4), None);
    assert_eq!(digit_pick(false, "3", 2), None);
    assert_eq!(digit_pick(true, "2", 4), None);

    let mut prompt = Prompt::new(setup(None));
    assert_eq!(
        prompt.update(Message::Input("3".into())),
        Some(Outcome::Answer {
            activity: act("Study: Rust book"),
            apply_late: true
        })
    );
}

#[test]
fn typed_digits_after_editing_are_text() {
    let mut prompt = Prompt::new(setup(None));
    prompt.update(Message::Input("Call".into()));
    assert_eq!(prompt.update(Message::Input("Call 2".into())), None);
}

#[test]
fn arrows_move_the_selection() {
    let mut prompt = Prompt::new(setup(None));
    prompt.key(&press(Key::Named(Named::ArrowDown), Modifiers::empty()));
    prompt.key(&press(Key::Named(Named::ArrowDown), Modifiers::empty()));
    prompt.key(&press(Key::Named(Named::ArrowUp), Modifiers::empty()));
    assert_eq!(
        prompt.update(Message::Submit),
        Some(Outcome::Answer {
            activity: act("Work: Meetings"),
            apply_late: true
        })
    );
}

#[test]
fn command_digit_picks_at_any_time() {
    let mut prompt = Prompt::new(setup(None));
    prompt.update(Message::Input("Work".into()));
    let outcome = prompt.key(&press(ch("2"), Modifiers::COMMAND));
    assert!(matches!(outcome, Some(Outcome::Answer { .. })));
}

#[test]
fn escape_skips_or_closes_more() {
    let mut prompt = Prompt::new(setup(None));
    let esc = press(Key::Named(Named::Escape), Modifiers::empty());
    prompt.key(&press(ch("k"), Modifiers::COMMAND));
    assert!(prompt.is_more_open());
    assert_eq!(prompt.key(&esc), None);
    assert!(!prompt.is_more_open());
    assert_eq!(prompt.key(&esc), Some(Outcome::Skip));
}

#[test]
fn more_menu_enter_keeps_for_an_hour_by_default() {
    let mut prompt = Prompt::new(setup(None));
    prompt.update(Message::ToggleMore);
    assert_eq!(
        prompt.update(Message::Submit),
        Some(Outcome::Keep {
            activity: act("Work: Code review"),
            until: KeepUntil::For(SignedDuration::from_hours(1))
        })
    );
}

#[test]
fn more_menu_arrows_reach_snooze_and_pause() {
    let mut prompt = Prompt::new(setup(None));
    prompt.update(Message::ToggleMore);
    for _ in 0..3 {
        prompt.key(&press(Key::Named(Named::ArrowDown), Modifiers::empty()));
    }
    assert_eq!(
        prompt.update(Message::Submit),
        Some(Outcome::Snooze(SignedDuration::from_mins(5)))
    );
    assert_eq!(prompt.update(Message::More(8)), Some(Outcome::Pause(None)));
}

#[test]
fn empty_answer_is_highlighted_not_logged() {
    let mut s = setup(None);
    s.previous = None;
    s.suggestions.clear();
    let mut prompt = Prompt::new(s);
    assert_eq!(prompt.update(Message::Submit), None);
    assert!(prompt.invalid);
    prompt.update(Message::Input("L".into()));
    assert!(!prompt.invalid);
}

#[test]
fn late_prompt_offers_the_whole_gap_and_can_be_unticked() {
    let mut prompt = Prompt::new(setup(Some(37)));
    assert_eq!(prompt.window_size().height, 380.0);
    assert_eq!(prompt.covered(), (at(0), at(37)));
    assert_eq!(prompt.primary_label(&prompt.rows()), "Log 37 min");
    prompt.key(&press(ch("l"), Modifiers::COMMAND));
    assert_eq!(prompt.covered(), (at(0), at(15)));
    assert_eq!(
        prompt.update(Message::Submit),
        Some(Outcome::Answer {
            activity: act("Work: Code review"),
            apply_late: false
        })
    );
}

#[test]
fn new_activity_changes_the_primary_label_and_keeps_room_for_the_hint() {
    let mut prompt = Prompt::new(setup(None));
    prompt.update(Message::Input("Study: Rust async".into()));
    let rows = prompt.rows();
    assert_eq!(prompt.primary_label(&rows), "Create & log");
    assert!(rows.len() <= 3);
}

#[test]
fn refresh_keeps_what_was_typed() {
    let mut prompt = Prompt::new(setup(None));
    prompt.update(Message::Input("Lun".into()));
    prompt.refresh(setup(Some(5)));
    assert_eq!(prompt.input, "Lun");
}

#[test]
fn enter_outside_the_field_still_logs() {
    let mut prompt = Prompt::new(setup(None));
    let enter = press(Key::Named(Named::Enter), Modifiers::empty());
    assert!(matches!(prompt.key(&enter), Some(Outcome::Answer { .. })));
    let captured = KeyPress {
        captured: true,
        ..enter
    };
    assert_eq!(
        prompt.key(&captured),
        None,
        "the field's on_submit handles it"
    );
}

#[test]
fn digits_type_normally_once_the_field_is_cleared() {
    let mut prompt = Prompt::new(setup(None));
    // Backspace over the selected pre-fill, then type a name starting with a digit.
    assert_eq!(prompt.update(Message::Input(String::new())), None);
    assert_eq!(prompt.update(Message::Input("1".into())), None);
    assert_eq!(prompt.update(Message::Input("1:1 with Sam".into())), None);
    assert_eq!(
        prompt.update(Message::Submit),
        Some(Outcome::Answer {
            activity: act("1:1 with Sam"),
            apply_late: true
        })
    );
}
