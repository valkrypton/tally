use iced::keyboard::Modifiers;
use jiff::civil::date;

use super::*;

fn t(h: i8, m: i8) -> Timestamp {
    date(2026, 10, 16)
        .at(h, m, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp()
}

fn act(name: &str) -> ActivityName {
    ActivityName::parse(name).unwrap()
}

fn block(id: ActivityId, name: &str) -> Block {
    Block {
        activity: id,
        name: act(name),
        start: t(11, 0),
        end: t(12, 0),
    }
}

fn row(id: ActivityId, name: &str, start: Timestamp, end: Timestamp) -> EntryRow {
    EntryRow {
        activity: id,
        name: act(name),
        start,
        end,
    }
}

fn calendar() -> Calendar {
    let today = date(2026, 10, 16);
    let mut c = Calendar::new(today, TimeZone::UTC);
    let empty = || Totals {
        groups: vec![],
        seconds: 0,
        first: None,
        last: None,
    };
    c.set_data(
        CalendarData {
            entries: vec![row(2, "Work: Meetings", t(11, 0), t(12, 0))],
            days: vec![(today, empty())],
            schedule: WeeklySchedule::default(),
            logged_until: t(12, 0),
        },
        today,
    );
    c
}

fn sample_data() -> CalendarData {
    CalendarData {
        entries: vec![row(2, "Work: Meetings", t(11, 0), t(12, 0))],
        days: vec![],
        schedule: WeeklySchedule::default(),
        logged_until: t(16, 0),
    }
}

fn key(named: Named) -> KeyPress {
    KeyPress::new(Key::Named(named), Modifiers::empty())
}

#[test]
fn opens_on_this_week_and_never_steps_into_the_future() {
    let mut c = calendar();
    assert_eq!(
        c.spec(),
        RangeSpec::new(RangeUnit::Week, date(2026, 10, 16))
    );
    assert_eq!(c.update(Message::Step(1)), None);
    assert_eq!(c.update(Message::Step(-1)), Some(Outcome::RangeChanged));
    assert_eq!(c.spec().anchor, date(2026, 10, 9));
}

#[test]
fn switching_units_keeps_the_viewed_date_and_open_day_jumps_to_it() {
    let mut c = calendar();
    assert_eq!(
        c.update(Message::Unit(RangeUnit::Month)),
        Some(Outcome::RangeChanged)
    );
    assert_eq!(
        c.spec(),
        RangeSpec::new(RangeUnit::Month, date(2026, 10, 16))
    );
    assert_eq!(c.update(Message::Unit(RangeUnit::Year)), None);
    c.update(Message::OpenDay(date(2026, 10, 12)));
    assert_eq!(c.spec(), RangeSpec::new(RangeUnit::Day, date(2026, 10, 12)));
}

#[test]
fn renaming_a_block_merges_only_after_confirming() {
    let mut c = calendar();
    assert_eq!(
        c.update(Message::SelectBlock(block(2, "Work: Meetings"))),
        Some(Outcome::Focus(editor_id()))
    );
    assert_eq!(c.editor_input(), Some("Work: Meetings"));
    c.update(Message::Input("Work: Email".into()));
    assert_eq!(
        c.update(Message::Save),
        Some(Outcome::Rename {
            id: 2,
            to: act("Work: Email")
        })
    );
    c.conflict(3);
    assert_eq!(
        c.update(Message::Save),
        Some(Outcome::Merge { from: 2, into: 3 })
    );
    c.saved(Ok(()));
    assert_eq!(c.editor_input(), None);
}

#[test]
fn logging_a_gap_needs_a_name() {
    let mut c = calendar();
    c.update(Message::SelectGap {
        start: t(11, 35),
        end: t(11, 50),
    });
    assert_eq!(c.update(Message::Save), None);
    c.update(Message::Input("Work: Email".into()));
    assert_eq!(
        c.update(Message::Save),
        Some(Outcome::LogTime {
            activity: act("Work: Email"),
            start: t(11, 35),
            end: t(11, 50)
        })
    );
}

#[test]
fn failed_saves_keep_the_editor_and_escape_closes_it() {
    let mut c = calendar();
    c.update(Message::SelectGap {
        start: t(11, 35),
        end: t(11, 50),
    });
    c.update(Message::Input("Lunch".into()));
    c.saved(Err("disk full".into()));
    assert_eq!(c.editor_input(), Some("Lunch"));
    assert_eq!(c.key(&key(Named::Escape)), None);
    assert_eq!(c.editor_input(), None);
    assert_eq!(c.key(&key(Named::ArrowLeft)), Some(Outcome::RangeChanged));
}

#[test]
fn every_unit_builds_a_view() {
    let mut c = calendar();
    for unit in [RangeUnit::Day, RangeUnit::Week, RangeUnit::Month] {
        c.update(Message::Unit(unit));
        c.set_data(sample_data(), date(2026, 10, 16));
        let _ = c.view();
    }
}

#[test]
fn day_gaps_end_at_logged_until() {
    let mut c = calendar();
    c.update(Message::OpenDay(date(2026, 10, 16)));
    c.set_data(sample_data(), date(2026, 10, 16)); // logged_until = 16:00
    let gaps: Vec<_> = c
        .day_slots()
        .iter()
        .filter_map(|s| match s {
            Slot::Gap { start, end } => Some((*start, *end)),
            _ => None,
        })
        .collect();
    assert_eq!(gaps, [(t(9, 0), t(11, 0)), (t(12, 0), t(16, 0))]);
}
