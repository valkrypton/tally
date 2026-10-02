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

#[test]
fn groups_activities_under_categories() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Development", at(0, 9, 0), at(0, 12, 30));
    record(&mut store, "Work: Meeting", at(0, 13, 15), at(0, 14, 5));
    record(&mut store, "Lunch", at(0, 12, 30), at(0, 13, 15));
    record(&mut store, "Study: Homework", at(0, 14, 5), at(0, 15, 20));

    let totals = store.totals(&day(0)).unwrap();
    let shape: Vec<_> = totals
        .groups
        .iter()
        .map(|g| {
            let names: Vec<_> = g.activities.iter().map(|a| a.name.name.as_str()).collect();
            (g.category.as_deref(), g.seconds / 60, names)
        })
        .collect();
    assert_eq!(
        shape,
        vec![
            (Some("Work"), 260, vec!["Development", "Meeting"]),
            (Some("Study"), 75, vec!["Homework"]),
            (None, 45, vec!["Lunch"]),
        ]
    );
    assert_eq!(totals.seconds, 380 * 60);
    assert_eq!(totals.first, Some(at(0, 9, 0)));
    assert_eq!(totals.last, Some(at(0, 15, 20)));
}

#[test]
fn entries_crossing_midnight_are_split_between_days() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Deploy", at(0, 23, 30), at(1, 0, 30));
    assert_eq!(store.totals(&day(0)).unwrap().seconds, 1800);
    assert_eq!(store.totals(&day(1)).unwrap().seconds, 1800);
    assert_eq!(store.totals(&day(1)).unwrap().first, Some(at(1, 0, 0)));
}

#[test]
fn days_follow_the_local_timezone_across_dst() {
    let tz = TimeZone::get("Europe/Berlin").unwrap();
    let mut store = Store::open_in_memory().unwrap();
    // 2026-10-25 is 25 hours long in Berlin; 22:30–23:30 UTC on the 25th is local 23:30–00:30.
    let start = date(2026, 10, 25)
        .at(22, 30, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp();
    let end = date(2026, 10, 25)
        .at(23, 30, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp();
    record(&mut store, "Late night", start, end);
    let dst_day = RangeSpec::new(RangeUnit::Day, date(2026, 10, 25))
        .resolve(&tz)
        .unwrap();
    assert_eq!(store.totals(&dst_day).unwrap().seconds, 1800);
}

#[test]
fn empty_range_has_no_groups() {
    let store = Store::open_in_memory().unwrap();
    let totals = store.totals(&day(0)).unwrap();
    assert!(totals.groups.is_empty());
    assert_eq!((totals.seconds, totals.first, totals.last), (0, None, None));
}
