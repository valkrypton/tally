//! CSV and JSON export of entries and corrections in a range.

use std::io::Write;

use jiff::Timestamp;
use jiff::civil::Date;
use rusqlite::params;
use serde::Serialize;

use crate::Result;
use crate::range::TimeRange;
use crate::store::{Store, day_key, from_day_key, from_secs, secs};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportRow {
    Entry {
        start: Timestamp,
        end: Timestamp,
        category: Option<String>,
        activity: String,
        source: String,
    },
    Correction {
        day: Date,
        category: Option<String>,
        activity: String,
        seconds: i64,
    },
}

impl ExportRow {
    pub fn seconds(&self) -> i64 {
        match self {
            ExportRow::Entry { start, end, .. } => end.duration_since(*start).as_secs(),
            ExportRow::Correction { seconds, .. } => *seconds,
        }
    }
}

impl Store {
    /// Entries (clipped to the range) by start time, then corrections by day.
    pub fn export_rows(&self, range: &TimeRange) -> Result<Vec<ExportRow>> {
        let (start, end) = (secs(range.start), secs(range.end));
        let mut stmt = self.conn().prepare(
            "SELECT MAX(e.start_at, ?1), MIN(e.end_at, ?2), c.name, a.name, e.source
             FROM entries e JOIN activities a ON a.id = e.activity_id
             LEFT JOIN categories c ON c.id = a.category_id
             WHERE e.end_at > ?1 AND e.start_at < ?2
             ORDER BY e.start_at, e.id",
        )?;
        let entries = stmt.query_map(params![start, end], |row| {
            Ok(ExportRow::Entry {
                start: from_secs(row.get(0)?),
                end: from_secs(row.get(1)?),
                category: row.get(2)?,
                activity: row.get(3)?,
                source: row.get(4)?,
            })
        })?;
        let mut rows = entries.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt = self.conn().prepare(
            "SELECT r.day, c.name, a.name, r.seconds
             FROM corrections r JOIN activities a ON a.id = r.activity_id
             LEFT JOIN categories c ON c.id = a.category_id
             WHERE r.day BETWEEN ?1 AND ?2
             ORDER BY r.day, r.id",
        )?;
        let corrections = stmt.query_map(
            params![day_key(range.first_day), day_key(range.last_day)],
            |row| {
                Ok(ExportRow::Correction {
                    day: from_day_key(row.get(0)?),
                    category: row.get(1)?,
                    activity: row.get(2)?,
                    seconds: row.get(3)?,
                })
            },
        )?;
        rows.extend(corrections.collect::<rusqlite::Result<Vec<_>>>()?);
        Ok(rows)
    }
}

/// Columns: kind, start, end, day, category, activity, minutes.
pub fn write_csv<W: Write>(rows: &[ExportRow], out: W) -> Result<()> {
    let mut csv = csv::Writer::from_writer(out);
    csv.write_record([
        "kind", "start", "end", "day", "category", "activity", "minutes",
    ])?;
    for row in rows {
        let minutes = format!("{:.2}", row.seconds() as f64 / 60.0);
        match row {
            ExportRow::Entry {
                start,
                end,
                category,
                activity,
                ..
            } => csv.write_record([
                "entry",
                &start.to_string(),
                &end.to_string(),
                "",
                category.as_deref().unwrap_or(""),
                activity,
                &minutes,
            ])?,
            ExportRow::Correction {
                day,
                category,
                activity,
                ..
            } => csv.write_record([
                "correction",
                "",
                "",
                &day.to_string(),
                category.as_deref().unwrap_or(""),
                activity,
                &minutes,
            ])?,
        }
    }
    csv.flush().map_err(csv::Error::from)?;
    Ok(())
}

pub fn to_json(rows: &[ExportRow]) -> Result<String> {
    Ok(serde_json::to_string_pretty(rows)?)
}

#[cfg(test)]
mod tests;
