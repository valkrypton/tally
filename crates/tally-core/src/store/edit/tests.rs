use super::super::test_support::*;
use super::*;
use crate::range::{RangeSpec, RangeUnit};
use jiff::civil::date;
use jiff::tz::TimeZone;

fn day(offset: i8) -> TimeRange {
    RangeSpec::new(RangeUnit::Day, date(2026, 10, 5 + offset))
        .resolve(&TimeZone::UTC)
        .unwrap()
}

fn setup() -> (Store, ActivityId) {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Dev", at(0, 9, 0), at(0, 10, 0));
    record(&mut store, "Work: Dev", at(0, 11, 0), at(0, 11, 30));
    let id = store.ensure_activity(&act("Work: Dev")).unwrap();
    (store, id)
}

fn entries(store: &Store) -> Vec<(i64, i64)> {
    let mut stmt = store
        .conn
        .prepare("SELECT start_at, end_at FROM entries ORDER BY start_at")
        .unwrap();
    stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn subtract_trims_latest_entries_first() {
    let (mut store, id) = setup();
    let today = date(2026, 10, 5);
    store
        .edit_total(id, &day(0), today, DurationEdit::Subtract(40 * 60))
        .unwrap();
    // 11:00–11:30 is removed entirely, then 10 minutes come off 09:00–10:00.
    assert_eq!(
        entries(&store),
        vec![(secs(at(0, 9, 0)), secs(at(0, 9, 50)))]
    );
    assert_eq!(store.activity_total(id, &day(0)).unwrap(), 50 * 60);
}

#[test]
fn subtracting_more_than_recorded_empties_the_day() {
    let (mut store, id) = setup();
    let today = date(2026, 10, 5);
    store
        .edit_total(id, &day(0), today, DurationEdit::Subtract(10 * 3600))
        .unwrap();
    assert_eq!(store.activity_total(id, &day(0)).unwrap(), 0);
}

#[test]
fn set_to_zero_removes_entries_and_corrections() {
    let (mut store, id) = setup();
    let today = date(2026, 10, 5);
    store
        .edit_total(id, &day(0), today, DurationEdit::Add(15 * 60))
        .unwrap();
    store
        .edit_total(id, &day(0), today, DurationEdit::Set(0))
        .unwrap();
    assert_eq!(store.activity_total(id, &day(0)).unwrap(), 0);
    assert!(entries(&store).is_empty());
}

#[test]
fn raising_a_total_adds_a_correction_for_the_difference() {
    let (mut store, id) = setup();
    let today = date(2026, 10, 5);
    store
        .edit_total(id, &day(0), today, DurationEdit::Set(2 * 3600))
        .unwrap();
    assert_eq!(store.activity_total(id, &day(0)).unwrap(), 2 * 3600);
    assert_eq!(entries(&store).len(), 2, "entries are untouched");
}

#[test]
fn trimming_falls_back_to_corrections() {
    let (mut store, id) = setup();
    let today = date(2026, 10, 5);
    store
        .edit_total(id, &day(0), today, DurationEdit::Add(30 * 60))
        .unwrap();
    store
        .edit_total(id, &day(0), today, DurationEdit::Subtract(100 * 60))
        .unwrap();
    // 90 minutes of entries are gone, plus 10 of the 30 correction minutes.
    assert_eq!(store.activity_total(id, &day(0)).unwrap(), 20 * 60);
}

#[test]
fn trimming_keeps_the_part_of_an_entry_outside_the_range() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Deploy", at(0, 23, 0), at(1, 1, 0));
    let id = store.ensure_activity(&act("Deploy")).unwrap();
    store
        .edit_total(id, &day(0), date(2026, 10, 5), DurationEdit::Set(0))
        .unwrap();
    assert_eq!(store.activity_total(id, &day(0)).unwrap(), 0);
    assert_eq!(store.activity_total(id, &day(1)).unwrap(), 3600);
}

#[test]
fn corrections_in_a_week_count_toward_that_week() {
    let (mut store, id) = setup();
    let week = RangeSpec::new(RangeUnit::Week, date(2026, 10, 5));
    let range = week.resolve(&TimeZone::UTC).unwrap();
    let day_for = week.correction_day(date(2026, 10, 20)).unwrap();
    store
        .edit_total(id, &range, day_for, DurationEdit::Add(3600))
        .unwrap();
    assert_eq!(day_for, date(2026, 10, 11));
    assert_eq!(store.activity_total(id, &day(6)).unwrap(), 3600);
}
