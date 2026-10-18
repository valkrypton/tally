use jiff::Timestamp;
use tally_core::ActivityName;
use tally_core::range::{RangeSpec, RangeUnit};
use tally_core::scheduler::{NewEntry, Source};
use tally_core::settings::WeeklySchedule;
use tally_core::store::Store;

use super::*;

fn regular(minute: i64) -> scheduler::Prompt {
    scheduler::Prompt::Regular {
        due_at: Timestamp::from_second(minute * 60).unwrap(),
    }
}

fn record() -> Action {
    Action::Record(NewEntry {
        activity: ActivityName::parse("Lunch").unwrap(),
        start: Timestamp::from_second(0).unwrap(),
        end: Timestamp::from_second(60).unwrap(),
        source: Source::Prompt,
    })
}

#[test]
fn the_last_prompt_action_wins() {
    assert_eq!(final_prompt_action(&[record()]), None);
    assert_eq!(
        final_prompt_action(&[Action::HidePrompt, record()]),
        Some(None)
    );
    assert_eq!(
        final_prompt_action(&[Action::HidePrompt, Action::ShowPrompt(regular(5))]),
        Some(Some(regular(5)))
    );
    assert_eq!(
        final_prompt_action(&[Action::ShowPrompt(regular(5)), Action::HidePrompt]),
        Some(None)
    );
}

#[test]
fn a_click_right_after_the_dashboard_closed_itself_does_not_reopen_it() {
    let closed = Instant::now();
    assert!(reopen_allowed(None, closed));
    assert!(!reopen_allowed(
        Some(closed),
        closed + Duration::from_millis(100)
    ));
    assert!(reopen_allowed(
        Some(closed),
        closed + Duration::from_millis(500)
    ));
}

/// A data folder with a schedule that always tracks and a prompt left open when tally
/// last quit, asking about the 15 minutes before `due`.
fn restored_app(name: &str, due: Timestamp) -> App {
    let dir = std::env::temp_dir().join(format!("tally-app-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let paths = Paths::in_dir(dir);
    paths.create_data_dir().unwrap();
    let store = Store::open(&paths.database).unwrap();
    let settings = Settings {
        schedule: tally_core::settings::WeeklySchedule::always(),
        ..Settings::default()
    };
    store.save_settings(&settings).unwrap();
    store
        .save_snapshot(&scheduler::Snapshot {
            state: State::Prompting(scheduler::Prompt::Regular { due_at: due }),
            cursor: due - jiff::SignedDuration::from_mins(15),
            last_tick: clock::now(),
            last_activity: Some(ActivityName::parse("Work: Code review").unwrap()),
        })
        .unwrap();
    drop(store);
    let (app, _) = App::boot(paths);
    assert!(app.core.is_some(), "store opens");
    app
}

#[test]
fn answering_a_restored_prompt_records_it_and_closes_the_prompt() {
    let due = clock::now() - jiff::SignedDuration::from_mins(1);
    let mut app = restored_app("answer", due);
    let visible = app
        .core
        .as_ref()
        .unwrap()
        .scheduler
        .visible_prompt()
        .cloned()
        .unwrap();
    let _ = app.show_prompt(visible, clock::now());
    assert!(app.windows.is_open(WindowKind::Prompt));

    // Enter logs the pre-filled previous answer.
    let _ = app.update(Message::Prompt(prompt::Message::Submit));

    assert!(app.prompt.is_none());
    assert!(!app.windows.is_open(WindowKind::Prompt));
    let core = app.core.as_ref().unwrap();
    let rows = core
        .store
        .export_rows(
            &tally_core::range::RangeSpec::new(
                tally_core::range::RangeUnit::All,
                app.today(clock::now()),
            )
            .resolve(&app.tz)
            .unwrap(),
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].seconds(), 15 * 60);
    assert!(matches!(core.scheduler.state(), State::Tracking { .. }));
    // The new scheduler state was saved, so a restart doesn't ask again.
    assert!(matches!(
        core.store.load_snapshot().unwrap().unwrap().state,
        State::Tracking { .. }
    ));
    std::fs::remove_dir_all(&app.paths.data_dir).unwrap();
}

#[test]
fn a_broken_data_file_opens_the_error_window_instead_of_a_fresh_database() {
    let dir = std::env::temp_dir().join(format!("tally-app-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let paths = Paths::in_dir(dir);
    // A folder where the database file should be: SQLite can't open it.
    std::fs::create_dir_all(&paths.database).unwrap();
    let (app, _) = App::boot(paths);
    assert!(app.core.is_none());
    assert!(app.windows.is_open(WindowKind::DataError));
    assert!(app.data_error.as_ref().unwrap().path.ends_with("tally.db"));
    std::fs::remove_dir_all(&app.paths.data_dir).unwrap();
}

fn utc(y: i16, mo: i8, d: i8, h: i8, mi: i8) -> Timestamp {
    jiff::civil::datetime(y, mo, d, h, mi, 0, 0)
        .to_zoned(TimeZone::UTC)
        .unwrap()
        .timestamp()
}

#[test]
fn calendar_data_has_a_totals_row_per_day_and_the_week_entries() {
    let mut store = Store::open_in_memory().unwrap();
    store
        .record(&NewEntry {
            activity: ActivityName::parse("Work: Code").unwrap(),
            start: utc(2026, 10, 12, 11, 0),
            end: utc(2026, 10, 12, 12, 0),
            source: Source::Prompt,
        })
        .unwrap();
    let spec = RangeSpec::new(RangeUnit::Week, jiff::civil::date(2026, 10, 14));
    let data = calendar_data(
        &store,
        spec,
        &TimeZone::UTC,
        &WeeklySchedule::default(),
        utc(2026, 10, 14, 9, 0),
    )
    .unwrap();
    assert_eq!(data.days.len(), 7);
    assert_eq!(data.days[0].0, jiff::civil::date(2026, 10, 12));
    assert_eq!(data.days[0].1.seconds, 3600);
    assert_eq!(data.entries.len(), 1);
}
