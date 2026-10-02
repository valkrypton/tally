//! Editing totals from the dashboard: trimming real entries and adding corrections.

use jiff::civil::Date;
use rusqlite::{Transaction, params};

use super::{ActivityId, Store, day_key, secs};
use crate::Result;
use crate::duration::DurationEdit;
use crate::range::TimeRange;

impl Store {
    /// Applies a typed edit to one activity's total in `range`.
    ///
    /// Lowering a total trims the activity's latest entries in the range, then its
    /// corrections. Raising it adds a correction filed under `correction_day`.
    pub fn edit_total(
        &mut self,
        activity: ActivityId,
        range: &TimeRange,
        correction_day: Date,
        edit: DurationEdit,
    ) -> Result<()> {
        let current = self.activity_total(activity, range)?;
        let change = match edit {
            DurationEdit::Set(target) => target - current,
            DurationEdit::Add(amount) => amount,
            DurationEdit::Subtract(amount) => -amount.min(current),
        };
        let tx = self.conn.transaction()?;
        if change < 0 {
            let left = trim_entries(&tx, activity, range, -change)?;
            trim_corrections(&tx, activity, range, left)?;
        } else if change > 0 {
            tx.execute(
                "INSERT INTO corrections (activity_id, day, seconds) VALUES (?1, ?2, ?3)",
                params![activity, day_key(correction_day), change],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
}

/// Removes up to `amount` seconds from the end of the activity's in-range entries,
/// latest first. Parts of an entry outside the range are kept. Returns what is left.
fn trim_entries(
    tx: &Transaction<'_>,
    activity: ActivityId,
    range: &TimeRange,
    amount: i64,
) -> Result<i64> {
    let (start, end) = (secs(range.start), secs(range.end));
    let entries: Vec<(i64, i64, i64, String)> = {
        let mut stmt = tx.prepare(
            "SELECT id, start_at, end_at, source FROM entries
             WHERE activity_id = ?1 AND end_at > ?2 AND start_at < ?3
             ORDER BY end_at DESC, id DESC",
        )?;
        stmt.query_map(params![activity, start, end], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?
    };

    let mut left = amount;
    for (id, entry_start, entry_end, source) in entries {
        if left == 0 {
            break;
        }
        let in_range_start = entry_start.max(start);
        let in_range_end = entry_end.min(end);
        let take = left.min(in_range_end - in_range_start);
        left -= take;

        tx.execute("DELETE FROM entries WHERE id = ?1", [id])?;
        let kept_end = in_range_end - take;
        let insert =
            "INSERT INTO entries (activity_id, start_at, end_at, source) VALUES (?1, ?2, ?3, ?4)";
        if kept_end > entry_start {
            tx.execute(insert, params![activity, entry_start, kept_end, source])?;
        }
        if entry_end > in_range_end {
            tx.execute(insert, params![activity, in_range_end, entry_end, source])?;
        }
    }
    Ok(left)
}

/// Removes up to `amount` seconds from the activity's corrections in range, latest day first.
fn trim_corrections(
    tx: &Transaction<'_>,
    activity: ActivityId,
    range: &TimeRange,
    amount: i64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let corrections: Vec<(i64, i64)> = {
        let mut stmt = tx.prepare(
            "SELECT id, seconds FROM corrections
             WHERE activity_id = ?1 AND day BETWEEN ?2 AND ?3
             ORDER BY day DESC, id DESC",
        )?;
        stmt.query_map(
            params![activity, day_key(range.first_day), day_key(range.last_day)],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?
        .collect::<rusqlite::Result<_>>()?
    };
    let mut left = amount;
    for (id, seconds) in corrections {
        if left == 0 {
            break;
        }
        if seconds <= left {
            tx.execute("DELETE FROM corrections WHERE id = ?1", [id])?;
            left -= seconds;
        } else {
            tx.execute(
                "UPDATE corrections SET seconds = ?1 WHERE id = ?2",
                [seconds - left, id],
            )?;
            left = 0;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
