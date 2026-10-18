use jiff::civil::{Time, date};
use tally_core::settings::WorkHours;
use tally_core::store::{ActivityTotal, Group};

use super::*;

fn t(day: i64, h: i64, m: i64) -> Timestamp {
    // 2026-10-05 00:00 UTC is a Monday.
    "2026-10-05T00:00:00Z".parse::<Timestamp>().unwrap()
        + SignedDuration::from_secs(day * 86400 + h * 3600 + m * 60)
}

fn row(id: ActivityId, name: &str, start: Timestamp, end: Timestamp) -> EntryRow {
    EntryRow {
        activity: id,
        name: ActivityName::parse(name).unwrap(),
        start,
        end,
    }
}

fn schedule() -> WeeklySchedule {
    let hours = Some(WorkHours {
        start: Time::constant(11, 0, 0, 0),
        end: Time::constant(20, 0, 0, 0),
    });
    WeeklySchedule {
        days: [hours, hours, hours, hours, hours, None, None],
    }
}

fn sample_totals() -> Totals {
    let total = |id, name: &str, seconds| ActivityTotal {
        id,
        name: ActivityName::parse(name).unwrap(),
        seconds,
    };
    Totals {
        groups: vec![
            Group {
                category: Some("Work".into()),
                seconds: 4 * 3600,
                activities: vec![
                    total(1, "Work: Code", 3 * 3600),
                    total(2, "Work: Email", 3600),
                ],
            },
            Group {
                category: None,
                seconds: 45 * 60,
                activities: vec![total(3, "Lunch", 45 * 60)],
            },
        ],
        seconds: 4 * 3600 + 45 * 60,
        first: None,
        last: None,
    }
}

#[test]
fn blocks_clip_to_the_day_and_merge_neighbours() {
    let entries = [
        row(1, "Work: Code", t(0, 9, 0), t(0, 9, 15)),
        row(1, "Work: Code", t(0, 9, 15), t(0, 9, 30)), // touches: merged
        row(1, "Work: Code", t(0, 9, 31), t(0, 9, 45)), // 1 min gap: merged
        row(2, "Lunch", t(0, 12, 0), t(0, 12, 30)),
        row(1, "Work: Code", t(0, 23, 50), t(1, 0, 20)), // crosses midnight
    ];
    let got: Vec<_> = blocks(&entries, date(2026, 10, 5), &TimeZone::UTC)
        .into_iter()
        .map(|b| (b.activity, b.start, b.end))
        .collect();
    assert_eq!(
        got,
        vec![
            (1, t(0, 9, 0), t(0, 9, 45)),
            (2, t(0, 12, 0), t(0, 12, 30)),
            (1, t(0, 23, 50), t(1, 0, 0))
        ]
    );
    assert_eq!(
        blocks(&entries, date(2026, 10, 6), &TimeZone::UTC)[0].start,
        t(1, 0, 0)
    );
}

#[test]
fn visible_hours_cover_schedule_and_entries() {
    let tz = TimeZone::UTC;
    let monday = date(2026, 10, 5);
    assert_eq!(
        visible_hours(&[(monday, vec![])], &schedule(), &tz),
        (11, 20)
    );
    let early = blocks(&[row(1, "A", t(0, 7, 40), t(0, 8, 10))], monday, &tz);
    assert_eq!(visible_hours(&[(monday, early)], &schedule(), &tz), (7, 20));
    let late = blocks(&[row(1, "A", t(0, 23, 0), t(1, 0, 30))], monday, &tz);
    assert_eq!(visible_hours(&[(monday, late)], &schedule(), &tz), (11, 24));
    let sunday = date(2026, 10, 11); // not a working day, nothing logged
    assert_eq!(
        visible_hours(&[(sunday, vec![])], &schedule(), &tz),
        DEFAULT_HOURS
    );
    assert_eq!(
        visible_hours(&[(monday, vec![]), (sunday, vec![])], &schedule(), &tz),
        (11, 20)
    );
    assert_eq!(visible_hours(&[], &schedule(), &tz), DEFAULT_HOURS);
}

#[test]
fn gaps_stop_at_logged_until() {
    let monday = date(2026, 10, 5);
    let b = blocks(
        &[row(1, "A", t(0, 11, 0), t(0, 12, 0))],
        monday,
        &TimeZone::UTC,
    );
    let working = Some((t(0, 11, 0), t(0, 20, 0)));
    let got = slots(&b, (t(0, 10, 0), t(0, 20, 0)), working, t(0, 14, 30));
    assert_eq!(
        got,
        vec![
            Slot::Empty {
                start: t(0, 10, 0),
                end: t(0, 11, 0)
            }, // before working hours
            Slot::Block(b[0].clone()),
            Slot::Gap {
                start: t(0, 12, 0),
                end: t(0, 14, 30)
            }, // loggable
            Slot::Empty {
                start: t(0, 14, 30),
                end: t(0, 20, 0)
            }, // not logged *yet*
        ]
    );
}

#[test]
fn slots_fill_a_dst_day_exactly() {
    let tz = TimeZone::get("Europe/London").unwrap();
    let day = date(2026, 10, 25); // clocks go back: 25 hours
    let (start, end) = window(day, (0, 24), &tz);
    assert_eq!(end.duration_since(start).as_hours(), 25);
    let total: u32 = slots(&[], (start, end), None, end)
        .iter()
        .map(|s| {
            let (a, b) = match s {
                Slot::Block(b) => (b.start, b.end),
                Slot::Gap { start, end } | Slot::Empty { start, end } => (*start, *end),
            };
            u32::from(portion(a, b))
        })
        .sum();
    assert_eq!(total, 25 * 60);
}

#[test]
fn slots_of_an_empty_window_are_empty() {
    assert!(slots(&[], (t(0, 9, 0), t(0, 9, 0)), None, t(0, 9, 0)).is_empty());
}

#[test]
fn category_colours_are_stable_and_ignore_case() {
    assert_eq!(category_color(Some("Work")), category_color(Some("work")));
    assert_eq!(category_color(None), theme::TEXT_MUTED);
}

#[test]
fn month_grid_runs_monday_to_sunday() {
    let oct = month_grid(date(2026, 10, 18));
    assert_eq!(
        (oct.len(), oct[0], oct[34]),
        (35, date(2026, 9, 28), date(2026, 11, 1))
    );
    let aug = month_grid(date(2026, 8, 1)); // starts on a Saturday
    assert_eq!(
        (aug.len(), aug[0], aug[41]),
        (42, date(2026, 7, 27), date(2026, 9, 6))
    );
}

#[test]
fn summary_lists_categories_and_the_top_activity() {
    // totals: Work 4h (Code 3h, Email 1h), Lunch 45m uncategorised
    let s = summary(&sample_totals());
    assert_eq!(s.seconds, 4 * 3600 + 45 * 60);
    assert_eq!(
        s.parts,
        vec![(Some("Work".into()), 4 * 3600), (None, 45 * 60)]
    );
    assert_eq!(s.top.as_deref(), Some("Code"));
}
