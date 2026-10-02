use jiff::Timestamp;
use tally_core::ActivityName;
use tally_core::scheduler::{Source, State};

use super::*;

#[derive(Default)]
struct FakeSink {
    fail_records: usize,
    fail_snapshots: usize,
    entries: Vec<NewEntry>,
    snapshots: Vec<Snapshot>,
}

impl WriteSink for FakeSink {
    fn record(&mut self, entry: &NewEntry) -> tally_core::Result<()> {
        if self.fail_records > 0 {
            self.fail_records -= 1;
            return Err(tally_core::Error::UnknownActivity(0));
        }
        self.entries.push(entry.clone());
        Ok(())
    }

    fn save_snapshot(&mut self, snapshot: &Snapshot) -> tally_core::Result<()> {
        if self.fail_snapshots > 0 {
            self.fail_snapshots -= 1;
            return Err(tally_core::Error::UnknownActivity(0));
        }
        self.snapshots.push(snapshot.clone());
        Ok(())
    }
}

fn entry(minute: i64) -> NewEntry {
    NewEntry {
        activity: ActivityName::parse("Work: Code").unwrap(),
        start: Timestamp::from_second(minute * 60).unwrap(),
        end: Timestamp::from_second(minute * 60 + 60).unwrap(),
        source: Source::Prompt,
    }
}

fn snapshot(cursor_minute: i64) -> Snapshot {
    let at = Timestamp::from_second(cursor_minute * 60).unwrap();
    Snapshot {
        state: State::OffSchedule,
        cursor: at,
        last_tick: at,
        last_activity: None,
    }
}

#[test]
fn flushes_entries_then_snapshot() {
    let mut pending = PendingWrites::default();
    let mut sink = FakeSink::default();
    pending.push_entry(entry(1));
    pending.push_entry(entry(2));
    pending.set_snapshot(snapshot(3));
    pending.flush(&mut sink);
    assert_eq!(sink.entries, vec![entry(1), entry(2)]);
    assert_eq!(sink.snapshots, vec![snapshot(3)]);
    assert_eq!(pending.unsaved_entries(), 0);
    assert!(!pending.is_failing());
}

#[test]
fn failed_entry_is_kept_and_blocks_the_snapshot() {
    let mut pending = PendingWrites::default();
    let mut sink = FakeSink {
        fail_records: 1,
        ..FakeSink::default()
    };
    pending.push_entry(entry(1));
    pending.push_entry(entry(2));
    pending.set_snapshot(snapshot(3));
    pending.flush(&mut sink);
    assert!(sink.entries.is_empty());
    assert!(
        sink.snapshots.is_empty(),
        "cursor must not run ahead of stored time"
    );
    assert_eq!(pending.unsaved_entries(), 2);
    assert!(pending.is_failing());
    assert!(pending.last_error().is_some());

    // Next tick: the store works again and everything goes in, in order.
    pending.flush(&mut sink);
    assert_eq!(sink.entries, vec![entry(1), entry(2)]);
    assert_eq!(sink.snapshots, vec![snapshot(3)]);
    assert!(!pending.is_failing());
}

#[test]
fn only_the_newest_snapshot_is_written() {
    let mut pending = PendingWrites::default();
    let mut sink = FakeSink {
        fail_snapshots: 1,
        ..FakeSink::default()
    };
    pending.set_snapshot(snapshot(1));
    pending.flush(&mut sink);
    assert!(pending.is_failing());
    pending.set_snapshot(snapshot(2));
    pending.flush(&mut sink);
    assert_eq!(sink.snapshots, vec![snapshot(2)]);
    assert!(!pending.is_failing());
}

#[test]
fn entries_survive_many_failed_ticks() {
    let mut pending = PendingWrites::default();
    let mut sink = FakeSink {
        fail_records: 50,
        ..FakeSink::default()
    };
    pending.push_entry(entry(1));
    for _ in 0..50 {
        pending.flush(&mut sink);
    }
    assert_eq!(pending.unsaved_entries(), 1);
    pending.flush(&mut sink);
    assert_eq!(sink.entries, vec![entry(1)]);
}
