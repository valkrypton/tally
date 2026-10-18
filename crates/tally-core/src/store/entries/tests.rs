use jiff::civil::date;
use jiff::tz::TimeZone;

use super::super::test_support::*;
use super::*;
use crate::range::{RangeSpec, RangeUnit, TimeRange};

fn day(offset: i8) -> TimeRange {
    RangeSpec::new(RangeUnit::Day, date(2026, 10, 5 + offset))
        .resolve(&TimeZone::UTC)
        .unwrap()
}

#[test]
fn entries_are_in_order_and_clipped_to_the_range() {
    let mut store = Store::open_in_memory().unwrap();
    record(&mut store, "Work: Code", at(0, 10, 0), at(0, 10, 15));
    record(&mut store, "Lunch", at(0, 9, 0), at(0, 9, 30));
    record(&mut store, "Late", at(0, 23, 50), at(1, 0, 20));
    let got: Vec<_> = store
        .entries(&day(0))
        .unwrap()
        .into_iter()
        .map(|r| (r.name.to_string(), r.start, r.end))
        .collect();
    assert_eq!(
        got,
        vec![
            ("Lunch".to_owned(), at(0, 9, 0), at(0, 9, 30)),
            ("Work: Code".to_owned(), at(0, 10, 0), at(0, 10, 15)),
            ("Late".to_owned(), at(0, 23, 50), at(1, 0, 0)),
        ]
    );
    let next = store.entries(&day(1)).unwrap();
    assert_eq!((next[0].start, next[0].end), (at(1, 0, 0), at(1, 0, 20)));
}
