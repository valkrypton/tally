use super::*;
use jiff::civil::date;

#[test]
fn week_runs_monday_to_sunday() {
    // 2026-10-08 is a Thursday.
    let spec = RangeSpec::new(RangeUnit::Week, date(2026, 10, 8));
    assert_eq!(
        spec.days().unwrap(),
        (date(2026, 10, 5), date(2026, 10, 11))
    );
}

#[test]
fn month_and_year_cover_whole_periods() {
    let month = RangeSpec::new(RangeUnit::Month, date(2026, 2, 14));
    assert_eq!(month.days().unwrap(), (date(2026, 2, 1), date(2026, 2, 28)));
    let year = RangeSpec::new(RangeUnit::Year, date(2026, 2, 14));
    assert_eq!(year.days().unwrap(), (date(2026, 1, 1), date(2026, 12, 31)));
}

#[test]
fn steps_move_by_unit() {
    let day = RangeSpec::new(RangeUnit::Day, date(2026, 10, 1));
    assert_eq!(day.step(-1).unwrap().anchor, date(2026, 9, 30));
    let month = RangeSpec::new(RangeUnit::Month, date(2026, 1, 31));
    assert_eq!(month.step(1).unwrap().days().unwrap().0, date(2026, 2, 1));
    let all = RangeSpec::new(RangeUnit::All, date(2026, 1, 31));
    assert_eq!(all.step(5).unwrap(), all);
}

#[test]
fn resolves_to_local_midnights_across_dst() {
    let tz = TimeZone::get("Europe/Berlin").unwrap();
    // Clocks go back on 2026-10-25, so that day is 25 hours long.
    let range = RangeSpec::new(RangeUnit::Day, date(2026, 10, 25))
        .resolve(&tz)
        .unwrap();
    assert_eq!(range.end.duration_since(range.start).as_hours(), 25);
}

#[test]
fn correction_day_prefers_today_inside_range() {
    let today = date(2026, 10, 8);
    let this_week = RangeSpec::new(RangeUnit::Week, today);
    assert_eq!(this_week.correction_day(today).unwrap(), today);
    let last_week = this_week.step(-1).unwrap();
    assert_eq!(last_week.correction_day(today).unwrap(), date(2026, 10, 4));
    let yesterday = RangeSpec::new(RangeUnit::Day, date(2026, 10, 7));
    assert_eq!(yesterday.correction_day(today).unwrap(), date(2026, 10, 7));
}
