use iced::keyboard::Modifiers;
use jiff::civil::date;
use tally_core::store::{ActivityTotal, Group};

use super::*;

const H: i64 = 3600;
const M: i64 = 60;

fn act(name: &str) -> ActivityName {
    ActivityName::parse(name).unwrap()
}

fn totals() -> Totals {
    let total = |id, name: &str, seconds| ActivityTotal {
        id,
        name: act(name),
        seconds,
    };
    Totals {
        groups: vec![
            Group {
                category: Some("Work".into()),
                seconds: 4 * H + 35 * M,
                activities: vec![
                    total(1, "Work: Code review", 2 * H + 40 * M),
                    total(2, "Work: Meetings", H + 10 * M),
                    total(3, "Work: Email", 45 * M),
                ],
            },
            Group {
                category: None,
                seconds: 45 * M,
                activities: vec![total(4, "Lunch", 45 * M)],
            },
        ],
        seconds: 5 * H + 20 * M,
        first: None,
        last: None,
    }
}

fn dashboard() -> Dashboard {
    let mut dashboard = Dashboard::new(date(2026, 10, 7), TimeZone::UTC);
    dashboard.set_data(
        totals(),
        Status {
            tracking: true,
            text: "Tracking".into(),
            figure: None,
        },
        false,
        date(2026, 10, 7),
    );
    dashboard
}

fn key(named: Named) -> KeyPress {
    KeyPress::new(Key::Named(named), Modifiers::empty())
}

#[test]
fn lines_put_activities_under_their_category() {
    let lines = lines(&totals());
    let kinds: Vec<LineKind> = lines.iter().map(|l| l.kind).collect();
    assert_eq!(
        kinds,
        [
            LineKind::Category,
            LineKind::Activity,
            LineKind::Activity,
            LineKind::Activity,
            LineKind::Single
        ]
    );
    assert_eq!(lines[0].activity, None, "category totals aren't editable");
    assert_eq!(lines[0].bar_max, 5 * H + 20 * M);
    assert_eq!(lines[1].name, "Code review");
    assert_eq!(
        lines[1].bar_max,
        4 * H + 35 * M,
        "activities compare to their category"
    );
    assert_eq!(lines[4].activity, Some(4));
    assert_eq!(lines[4].bar_max, 5 * H + 20 * M);
}

#[test]
fn presets_and_their_check_mark() {
    let today = date(2026, 10, 7);
    assert_eq!(
        preset_spec(Preset::Yesterday, today),
        RangeSpec::new(RangeUnit::Day, date(2026, 10, 6))
    );
    let last_week = preset_spec(Preset::LastWeek, today);
    assert_eq!(last_week.days().unwrap().0, date(2026, 9, 28));
    assert_eq!(current_preset(last_week, today), Some(Preset::LastWeek));
    // Any day of this week is "This week".
    assert_eq!(
        current_preset(RangeSpec::new(RangeUnit::Week, date(2026, 10, 5)), today),
        Some(Preset::ThisWeek)
    );
    assert_eq!(
        current_preset(RangeSpec::new(RangeUnit::Month, today), today),
        None
    );
}

#[test]
fn cannot_step_into_the_future_or_move_all() {
    let today = date(2026, 10, 7);
    assert!(!can_step(RangeSpec::new(RangeUnit::Day, today), 1, today));
    assert!(can_step(RangeSpec::new(RangeUnit::Day, today), -1, today));
    assert!(can_step(
        RangeSpec::new(RangeUnit::Day, date(2026, 10, 6)),
        1,
        today
    ));
    assert!(!can_step(RangeSpec::new(RangeUnit::Week, today), 1, today));
    assert!(!can_step(RangeSpec::new(RangeUnit::All, today), -1, today));
}

#[test]
fn previews_follow_the_spec_rules() {
    let current = 2 * H + 40 * M;
    assert_eq!(preview(current, parse_edit("-30").unwrap()), 2 * H + 10 * M);
    assert_eq!(preview(current, parse_edit("+15").unwrap()), 2 * H + 55 * M);
    assert_eq!(preview(current, parse_edit("1h30").unwrap()), H + 30 * M);
    assert_eq!(preview(current, parse_edit("-5h").unwrap()), 0);
    assert_eq!(preview(current, parse_edit("0").unwrap()), 0);
}

#[test]
fn add_durations_must_be_positive() {
    assert_eq!(parse_add_duration("1h 30"), Some(90 * M));
    assert_eq!(parse_add_duration("+15"), Some(15 * M));
    assert_eq!(parse_add_duration("0"), None);
    assert_eq!(parse_add_duration("-30"), None);
    assert_eq!(parse_add_duration("soon"), None);
}

#[test]
fn editing_a_total_emits_the_parsed_edit() {
    let mut dashboard = dashboard();
    assert_eq!(
        dashboard.update(Message::EditTotal(1)),
        Some(Outcome::Focus(edit_id()))
    );
    dashboard.update(Message::EditInput("-30".into()));
    assert_eq!(
        dashboard.update(Message::SaveEdit),
        Some(Outcome::EditTotal {
            activity: 1,
            edit: DurationEdit::Subtract(30 * M)
        })
    );
    dashboard.saved(Ok(()));
    assert!(dashboard.edit.is_none());
}

#[test]
fn unparseable_edits_are_flagged_and_not_saved() {
    let mut dashboard = dashboard();
    dashboard.update(Message::EditTotal(2));
    dashboard.update(Message::EditInput("lots".into()));
    assert_eq!(dashboard.update(Message::SaveEdit), None);
    assert!(dashboard.edit.as_ref().unwrap().invalid);
}

#[test]
fn failed_saves_keep_the_editor_open() {
    let mut dashboard = dashboard();
    dashboard.update(Message::EditTotal(2));
    dashboard.update(Message::EditInput("+15".into()));
    dashboard.saved(Err("disk full".into()));
    assert_eq!(dashboard.edit.as_ref().unwrap().input, "+15");
    assert_eq!(dashboard.error.as_deref(), Some("disk full"));
}

#[test]
fn adding_time_needs_a_name_and_a_duration() {
    let mut dashboard = dashboard();
    dashboard.update(Message::StartAdd);
    dashboard.update(Message::AddDuration("45".into()));
    assert_eq!(dashboard.update(Message::SaveAdd), None);
    dashboard.update(Message::AddActivity("Work: Planning".into()));
    assert_eq!(
        dashboard.update(Message::SaveAdd),
        Some(Outcome::AddTime {
            activity: act("Work: Planning"),
            seconds: 45 * M
        })
    );
}

#[test]
fn escape_backs_out_one_level_then_closes() {
    let mut dashboard = dashboard();
    dashboard.update(Message::ToggleRangeMenu);
    dashboard.update(Message::EditTotal(1));
    assert_eq!(dashboard.key(&key(Named::Escape)), None);
    assert!(!dashboard.range_menu);
    assert_eq!(dashboard.key(&key(Named::Escape)), None);
    assert!(dashboard.edit.is_none());
    assert_eq!(dashboard.key(&key(Named::Escape)), Some(Outcome::Close));
}

#[test]
fn arrows_step_and_select_and_enter_edits() {
    let mut dashboard = dashboard();
    assert_eq!(
        dashboard.key(&key(Named::ArrowLeft)),
        Some(Outcome::RangeChanged)
    );
    assert_eq!(dashboard.spec().anchor, date(2026, 10, 6));
    dashboard.key(&key(Named::ArrowDown));
    dashboard.key(&key(Named::ArrowDown));
    assert_eq!(
        dashboard.key(&key(Named::Enter)),
        Some(Outcome::Focus(edit_id()))
    );
    assert_eq!(dashboard.edit.as_ref().unwrap().activity, 2);
}

#[test]
fn unit_switch_keeps_the_viewed_day() {
    let mut dashboard = dashboard();
    dashboard.update(Message::Step(-1));
    dashboard.update(Message::Unit(RangeUnit::Week));
    assert_eq!(
        dashboard.spec(),
        RangeSpec::new(RangeUnit::Week, date(2026, 10, 6))
    );
}

#[test]
fn corrections_go_to_the_viewed_day_or_today() {
    let mut dashboard = dashboard();
    dashboard.update(Message::Step(-1));
    assert_eq!(dashboard.correction_day(), date(2026, 10, 6));
    dashboard.update(Message::Unit(RangeUnit::Week));
    assert_eq!(dashboard.correction_day(), date(2026, 10, 7));
}
