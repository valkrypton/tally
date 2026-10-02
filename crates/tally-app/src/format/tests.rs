use jiff::civil::date;
use jiff::tz::TimeZone;
use tally_core::range::{RangeSpec, RangeUnit};

use super::*;

fn utc() -> TimeZone {
    TimeZone::UTC
}

fn at(day: i8, h: i8, m: i8) -> Timestamp {
    date(2026, 10, day)
        .at(h, m, 0, 0)
        .to_zoned(utc())
        .unwrap()
        .timestamp()
}

#[test]
fn durations_use_hours_and_minutes() {
    assert_eq!(duration(6 * 3600 + 35 * 60), "6h 35m");
    assert_eq!(duration(45 * 60), "0h 45m");
    assert_eq!(duration(-5), "0h 00m");
}

#[test]
fn short_durations_switch_at_an_hour() {
    assert_eq!(short_duration(37 * 60 + 59), "37 min");
    assert_eq!(short_duration(3600), "1h 00m");
}

#[test]
fn windows_and_spans() {
    let tz = utc();
    assert_eq!(window(at(5, 9, 0), at(5, 9, 15), &tz), "09:00 – 09:15");
    assert_eq!(span(at(5, 9, 0), at(5, 16, 35), &tz), "09:00 – 16:35");
    assert_eq!(
        span(at(5, 9, 0), at(7, 16, 35), &tz),
        "Mon 09:00 – Wed 16:35"
    );
}

#[test]
fn clock_uses_the_given_zone() {
    let karachi = TimeZone::get("Asia/Karachi").unwrap();
    assert_eq!(clock(at(5, 9, 0), &karachi), "14:00");
}

#[test]
fn day_titles() {
    let today = date(2026, 10, 5);
    let day = |d| range_title(RangeSpec::new(RangeUnit::Day, d), today);
    assert_eq!(day(today), ("Today".into(), "Monday 5 October".into()));
    assert_eq!(
        day(date(2026, 10, 4)),
        ("Yesterday".into(), "Sunday 4 October".into())
    );
    assert_eq!(
        day(date(2026, 10, 1)),
        ("Thursday".into(), "Thursday 1 October".into())
    );
    assert_eq!(day(date(2025, 12, 31)).1, "Wednesday 31 December 2025");
}

#[test]
fn week_month_year_titles() {
    let today = date(2026, 10, 7);
    let title = |unit, d| range_title(RangeSpec::new(unit, d), today);
    assert_eq!(
        title(RangeUnit::Week, today),
        ("This week".into(), "5 – 11 October".into())
    );
    assert_eq!(
        title(RangeUnit::Week, date(2026, 9, 30)),
        ("Last week".into(), "28 September – 4 October".into())
    );
    assert_eq!(
        title(RangeUnit::Week, date(2026, 9, 16)).0,
        "Week of 14 September"
    );
    assert_eq!(
        title(RangeUnit::Month, today),
        ("This month".into(), "October 2026".into())
    );
    assert_eq!(
        title(RangeUnit::Month, date(2026, 8, 3)),
        ("August".into(), "August 2026".into())
    );
    assert_eq!(
        title(RangeUnit::Year, today),
        ("This year".into(), "2026".into())
    );
    assert_eq!(
        title(RangeUnit::Year, date(2025, 1, 1)),
        ("2025".into(), "2025".into())
    );
    assert_eq!(title(RangeUnit::All, today).0, "All time");
}

#[test]
fn day_span_across_years() {
    let today = date(2026, 1, 2);
    assert_eq!(
        day_span(date(2025, 12, 29), date(2026, 1, 4), today),
        "29 December 2025 – 4 January"
    );
}

#[test]
fn highlight_marks_every_needle_case_insensitively() {
    let runs = highlight("Rust book", &["rust".into()]);
    assert_eq!(runs, vec![("Rust".into(), true), (" book".into(), false)]);
    let runs = highlight("Code review", &["e".into()]);
    assert_eq!(runs.iter().filter(|(_, on)| *on).count(), 3);
    assert_eq!(highlight("Lunch", &[]), vec![("Lunch".into(), false)]);
}
