//! Logged entries in a range, as drawn on the calendar.

use jiff::Timestamp;
use rusqlite::params;

use super::{ActivityId, Store, from_secs, secs};
use crate::range::TimeRange;
use crate::{ActivityName, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryRow {
    pub activity: ActivityId,
    pub name: ActivityName,
    pub start: Timestamp,
    pub end: Timestamp,
}

impl Store {
    /// Entries overlapping `range`, clipped to it, ordered by start. Corrections have no time of day.
    pub fn entries(&self, range: &TimeRange) -> Result<Vec<EntryRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, c.name, a.name, MAX(e.start_at, ?1), MIN(e.end_at, ?2)
             FROM entries e
             JOIN activities a ON a.id = e.activity_id
             LEFT JOIN categories c ON c.id = a.category_id
             WHERE e.end_at > ?1 AND e.start_at < ?2
             ORDER BY e.start_at",
        )?;
        let rows = stmt.query_map(params![secs(range.start), secs(range.end)], |row| {
            Ok(EntryRow {
                activity: row.get(0)?,
                name: super::activities::name_of(row, 1, 2)?,
                start: from_secs(row.get(3)?),
                end: from_secs(row.get(4)?),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

#[cfg(test)]
mod tests;
