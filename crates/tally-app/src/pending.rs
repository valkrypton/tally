//! Scheduler writes that haven't reached the database yet (spec §7: a failed write is
//! kept in memory and retried on every tick, never silently dropped).

use std::collections::VecDeque;

use tally_core::scheduler::{NewEntry, Snapshot};
use tally_core::store::Store;

/// Where pending writes go. `Store` in the app; a fake in tests.
pub trait WriteSink {
    fn record(&mut self, entry: &NewEntry) -> tally_core::Result<()>;
    fn save_snapshot(&mut self, snapshot: &Snapshot) -> tally_core::Result<()>;
}

impl WriteSink for Store {
    fn record(&mut self, entry: &NewEntry) -> tally_core::Result<()> {
        Store::record(self, entry)
    }

    fn save_snapshot(&mut self, snapshot: &Snapshot) -> tally_core::Result<()> {
        Store::save_snapshot(self, snapshot)
    }
}

#[derive(Debug, Default)]
pub struct PendingWrites {
    entries: VecDeque<NewEntry>,
    snapshot: Option<Snapshot>,
    last_error: Option<String>,
}

impl PendingWrites {
    pub fn push_entry(&mut self, entry: NewEntry) {
        self.entries.push_back(entry);
    }

    /// Only the newest snapshot matters, so this replaces any unsaved one.
    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshot = Some(snapshot);
    }

    /// Writes entries oldest first, stopping at the first failure. The snapshot is
    /// saved only once every entry is in: a saved cursor must never claim time that
    /// isn't stored yet, or a crash would lose it.
    pub fn flush(&mut self, sink: &mut impl WriteSink) {
        while let Some(entry) = self.entries.front() {
            match sink.record(entry) {
                Ok(()) => {
                    self.entries.pop_front();
                }
                Err(error) => {
                    tracing::warn!(%error, pending = self.entries.len(), "could not record entry");
                    self.last_error = Some(error.to_string());
                    return;
                }
            }
        }
        if let Some(snapshot) = &self.snapshot {
            if let Err(error) = sink.save_snapshot(snapshot) {
                tracing::warn!(%error, "could not save scheduler state");
                self.last_error = Some(error.to_string());
                return;
            }
            self.snapshot = None;
        }
        self.last_error = None;
    }

    /// Entries waiting to be written.
    pub fn unsaved_entries(&self) -> usize {
        self.entries.len()
    }

    /// True when the last flush failed: drives the tray badge and the main-window notice.
    pub fn is_failing(&self) -> bool {
        self.last_error.is_some()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

#[cfg(test)]
mod tests;
