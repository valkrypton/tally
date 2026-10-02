//! SQLite storage for activities, entries, corrections, settings and scheduler state.

mod activities;
mod edit;
mod totals;

use std::path::Path;

use jiff::Timestamp;
use jiff::civil::Date;
use rusqlite::{Connection, OptionalExtension, params};
use rusqlite_migration::{M, Migrations};

use crate::Result;
use crate::scheduler::{NewEntry, Snapshot};
use crate::settings::Settings;

pub use activities::{ActivityRow, Rename};
pub use totals::{ActivityTotal, Group, Totals};

pub type ActivityId = i64;

const MIGRATIONS: &[M<'static>] = &[M::up(include_str!("migrations/001_initial.sql"))];

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "foreign_keys", true)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        Migrations::from_slice(MIGRATIONS).to_latest(&mut conn)?;
        Ok(Self { conn })
    }

    pub fn record(&mut self, entry: &NewEntry) -> Result<()> {
        let tx = self.conn.transaction()?;
        let activity = activities::ensure(&tx, &entry.activity)?;
        tx.execute(
            "INSERT INTO entries (activity_id, start_at, end_at, source) VALUES (?1, ?2, ?3, ?4)",
            params![
                activity,
                secs(entry.start),
                secs(entry.end),
                entry.source.as_str()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn load_settings(&self) -> Result<Settings> {
        // Unreadable settings (e.g. hand-edited) fall back to defaults rather than
        // stopping the app from starting.
        Ok(self
            .get_kv("settings", "settings")?
            .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
            .map(Settings::normalized)
            .unwrap_or_default())
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        self.set_kv("settings", "settings", &serde_json::to_string(settings)?)
    }

    pub fn load_snapshot(&self) -> Result<Option<Snapshot>> {
        // A snapshot from an older version that no longer parses is discarded: the
        // scheduler then starts fresh, which only loses the in-progress interval.
        Ok(self
            .get_kv("runtime", "scheduler")?
            .and_then(|json| serde_json::from_str(&json).ok()))
    }

    pub fn save_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        self.set_kv("runtime", "scheduler", &serde_json::to_string(snapshot)?)
    }

    pub(crate) fn conn(&self) -> &Connection {
        &self.conn
    }

    fn get_kv(&self, table: &str, key: &str) -> Result<Option<String>> {
        let sql = format!("SELECT value FROM {table} WHERE key = ?1");
        Ok(self
            .conn
            .query_row(&sql, [key], |row| row.get(0))
            .optional()?)
    }

    fn set_kv(&self, table: &str, key: &str, value: &str) -> Result<()> {
        let sql = format!(
            "INSERT INTO {table} (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value"
        );
        self.conn.execute(&sql, [key, value])?;
        Ok(())
    }
}

/// Timestamps are stored as whole Unix seconds.
fn secs(ts: Timestamp) -> i64 {
    ts.as_second()
}

fn from_secs(seconds: i64) -> Timestamp {
    Timestamp::from_second(seconds).expect("stored timestamps are in range")
}

/// Dates are stored as `yyyymmdd` integers, which sort correctly (even for `Date::MIN`).
pub(crate) fn day_key(date: Date) -> i64 {
    i64::from(date.year()) * 10_000 + i64::from(date.month()) * 100 + i64::from(date.day())
}

pub(crate) fn from_day_key(key: i64) -> Date {
    let year = key.div_euclid(10_000);
    let rest = key.rem_euclid(10_000);
    Date::new(year as i16, (rest / 100) as i8, (rest % 100) as i8).expect("stored dates are valid")
}

#[cfg(test)]
pub(crate) mod test_support {
    use jiff::Timestamp;
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::Store;
    use crate::ActivityName;
    use crate::scheduler::{NewEntry, Source};

    /// 2026-10-05 (Monday) + `day_offset` days, at `h:m` UTC.
    pub fn at(day_offset: i8, h: i8, m: i8) -> Timestamp {
        date(2026, 10, 5 + day_offset)
            .at(h, m, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap()
            .timestamp()
    }

    pub fn act(name: &str) -> ActivityName {
        ActivityName::parse(name).unwrap()
    }

    pub fn record(store: &mut Store, name: &str, start: Timestamp, end: Timestamp) {
        store
            .record(&NewEntry {
                activity: act(name),
                start,
                end,
                source: Source::Prompt,
            })
            .unwrap();
    }
}

#[cfg(test)]
mod tests;
