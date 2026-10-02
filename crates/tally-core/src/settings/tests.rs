use super::*;
use jiff::civil::date;

fn utc() -> TimeZone {
    TimeZone::UTC
}

fn at(y: i16, m: i8, d: i8, h: i8, min: i8) -> Timestamp {
    date(y, m, d)
        .at(h, min, 0, 0)
        .to_zoned(utc())
        .unwrap()
        .timestamp()
}

#[test]
fn default_schedule_is_weekdays_nine_to_six() {
    let schedule = WeeklySchedule::default();
    // 2026-10-05 is a Monday.
    let period = schedule.period_containing(at(2026, 10, 5, 10, 0), &utc());
    assert_eq!(
        period,
        Some((at(2026, 10, 5, 9, 0), at(2026, 10, 5, 18, 0)))
    );
    assert_eq!(
        schedule.period_containing(at(2026, 10, 5, 8, 59), &utc()),
        None
    );
    assert_eq!(
        schedule.period_containing(at(2026, 10, 5, 18, 0), &utc()),
        None
    );
    // Saturday.
    assert_eq!(
        schedule.period_containing(at(2026, 10, 10, 10, 0), &utc()),
        None
    );
}

#[test]
fn inverted_hours_mean_no_period() {
    let mut schedule = WeeklySchedule::default();
    schedule.days[0] = Some(WorkHours {
        start: Time::constant(18, 0, 0, 0),
        end: Time::constant(9, 0, 0, 0),
    });
    assert_eq!(
        schedule.period_containing(at(2026, 10, 5, 10, 0), &utc()),
        None
    );
}

#[test]
fn normalizes_out_of_range_values() {
    let settings = Settings {
        interval_minutes: 1,
        idle_threshold_minutes: 0,
        ..Settings::default()
    }
    .normalized();
    assert_eq!(settings.interval_minutes, 5);
    assert_eq!(settings.idle_threshold_minutes, 1);
}

#[test]
fn missing_fields_use_defaults_when_deserializing() {
    let settings: Settings = serde_json::from_str(r#"{"interval_minutes": 20}"#).unwrap();
    assert_eq!(settings.interval_minutes, 20);
    assert_eq!(settings.idle_threshold_minutes, 10);
}
