use super::test_support::*;
use super::*;
use crate::scheduler::{NewEntry, Source, State};
use jiff::civil::date;

#[test]
fn day_keys_round_trip_and_sort() {
    for d in [date(2026, 10, 5), Date::MIN, Date::MAX, date(-1, 12, 31)] {
        assert_eq!(from_day_key(day_key(d)), d);
    }
    assert!(day_key(Date::MIN) < day_key(date(2026, 1, 1)));
    assert!(day_key(date(2026, 1, 31)) < day_key(date(2026, 2, 1)));
}

#[test]
fn settings_default_until_saved() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.load_settings().unwrap(), Settings::default());
    let changed = Settings {
        interval_minutes: 20,
        ..Settings::default()
    };
    store.save_settings(&changed).unwrap();
    assert_eq!(store.load_settings().unwrap(), changed);
}

#[test]
fn snapshot_round_trips() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(store.load_snapshot().unwrap(), None);
    let snapshot = Snapshot {
        state: State::Paused { until: None },
        cursor: at(0, 9, 0),
        last_tick: at(0, 9, 1),
        last_activity: Some(act("Work: Code")),
    };
    store.save_snapshot(&snapshot).unwrap();
    store.save_snapshot(&snapshot).unwrap();
    assert_eq!(store.load_snapshot().unwrap(), Some(snapshot));
}

#[test]
fn unreadable_settings_and_snapshot_fall_back() {
    let store = Store::open_in_memory().unwrap();
    store.set_kv("settings", "settings", "{not json").unwrap();
    store
        .set_kv("runtime", "scheduler", r#"{"state":"Teleporting"}"#)
        .unwrap();
    assert_eq!(store.load_settings().unwrap(), Settings::default());
    assert_eq!(store.load_snapshot().unwrap(), None);
}

#[test]
fn data_survives_reopening_the_file() {
    let dir = std::env::temp_dir().join(format!("tally-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("tally.db");
    let _ = std::fs::remove_file(&path);
    {
        let mut store = Store::open(&path).unwrap();
        record(&mut store, "Lunch", at(0, 12, 0), at(0, 12, 30));
    }
    let store = Store::open(&path).unwrap();
    assert_eq!(store.suggestions(10).unwrap(), vec![act("Lunch")]);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn version_1_files_migrate_and_accept_manual_entries() {
    let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    rusqlite_migration::Migrations::from_slice(&MIGRATIONS[..1])
        .to_latest(&mut conn)
        .unwrap();
    conn.execute_batch(
        "INSERT INTO activities (id, name) VALUES (1, 'Lunch');
         INSERT INTO entries (activity_id, start_at, end_at, source) VALUES (1, 100, 200, 'prompt');",
    )
    .unwrap();
    let mut store = Store::init(conn).unwrap();
    store
        .record(&NewEntry {
            activity: act("Lunch"),
            start: from_secs(300),
            end: from_secs(400),
            source: Source::Manual,
        })
        .unwrap();
    let sources: Vec<String> = store
        .conn()
        .prepare("SELECT source FROM entries ORDER BY start_at")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(sources, ["prompt", "manual"]);
}
