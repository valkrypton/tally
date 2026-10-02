use super::*;
use crate::duration::DurationEdit;
use crate::range::{RangeSpec, RangeUnit};
use crate::store::test_support::*;
use jiff::civil::date;
use jiff::tz::TimeZone;

fn sample() -> (Store, TimeRange) {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Meeting", at(0, 9, 0), at(0, 9, 45));
    record(
        &mut store,
        "Lunch, with \"team\"",
        at(0, 12, 0),
        at(0, 12, 30),
    );
    let range = RangeSpec::new(RangeUnit::Day, date(2026, 10, 5))
        .resolve(&TimeZone::UTC)
        .unwrap();
    let id = store.ensure_activity(&act("Work: Meeting")).unwrap();
    store
        .edit_total(id, &range, date(2026, 10, 5), DurationEdit::Add(15 * 60))
        .unwrap();
    (store, range)
}

#[test]
fn csv_matches_golden_output() {
    let (store, range) = sample();
    let mut out = Vec::new();
    write_csv(&store.export_rows(&range).unwrap(), &mut out).unwrap();
    let expected = "\
kind,start,end,day,category,activity,minutes
entry,2026-10-05T09:00:00Z,2026-10-05T09:45:00Z,,Work,Meeting,45.00
entry,2026-10-05T12:00:00Z,2026-10-05T12:30:00Z,,,\"Lunch, with \"\"team\"\"\",30.00
correction,,,2026-10-05,Work,Meeting,15.00
";
    assert_eq!(String::from_utf8(out).unwrap(), expected);
}

#[test]
fn json_matches_golden_output() {
    let (store, range) = sample();
    let json = to_json(&store.export_rows(&range).unwrap()).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    let expected = serde_json::json!([
        {
            "kind": "entry",
            "start": "2026-10-05T09:00:00Z",
            "end": "2026-10-05T09:45:00Z",
            "category": "Work",
            "activity": "Meeting",
            "source": "prompt"
        },
        {
            "kind": "entry",
            "start": "2026-10-05T12:00:00Z",
            "end": "2026-10-05T12:30:00Z",
            "category": null,
            "activity": "Lunch, with \"team\"",
            "source": "prompt"
        },
        {
            "kind": "correction",
            "day": "2026-10-05",
            "category": "Work",
            "activity": "Meeting",
            "seconds": 900
        }
    ]);
    assert_eq!(parsed, expected);
}

#[test]
fn entries_are_clipped_to_the_range() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Deploy", at(0, 23, 0), at(1, 1, 0));
    let range = RangeSpec::new(RangeUnit::Day, date(2026, 10, 6))
        .resolve(&TimeZone::UTC)
        .unwrap();
    let rows = store.export_rows(&range).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seconds(), 3600);
}
