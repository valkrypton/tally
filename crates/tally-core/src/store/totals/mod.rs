//! Totals per category and activity over a range, as shown on the dashboard.

use jiff::Timestamp;
use rusqlite::params;

use super::{ActivityId, Store, day_key, from_secs, secs};
use crate::range::TimeRange;
use crate::{ActivityName, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityTotal {
    pub id: ActivityId,
    pub name: ActivityName,
    pub seconds: i64,
}

/// A category with its activities, or a single uncategorised activity (`category: None`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub category: Option<String>,
    pub seconds: i64,
    pub activities: Vec<ActivityTotal>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Totals {
    /// Largest first.
    pub groups: Vec<Group>,
    pub seconds: i64,
    /// First and last recorded instants in the range (corrections have no time of day).
    pub first: Option<Timestamp>,
    pub last: Option<Timestamp>,
}

impl Store {
    pub fn totals(&self, range: &TimeRange) -> Result<Totals> {
        let (start, end) = (secs(range.start), secs(range.end));
        let (first_day, last_day) = (day_key(range.first_day), day_key(range.last_day));
        let mut stmt = self.conn.prepare(
            "SELECT a.id, c.name, a.name, SUM(t.seconds) FROM (
                 SELECT activity_id, MIN(end_at, ?2) - MAX(start_at, ?1) AS seconds
                 FROM entries WHERE end_at > ?1 AND start_at < ?2
                 UNION ALL
                 SELECT activity_id, seconds FROM corrections WHERE day BETWEEN ?3 AND ?4
             ) t
             JOIN activities a ON a.id = t.activity_id
             LEFT JOIN categories c ON c.id = a.category_id
             GROUP BY a.id
             HAVING SUM(t.seconds) > 0",
        )?;
        let rows = stmt.query_map(params![start, end, first_day, last_day], |row| {
            Ok(ActivityTotal {
                id: row.get(0)?,
                name: super::activities::name_of(row, 1, 2)?,
                seconds: row.get(3)?,
            })
        })?;

        let mut groups: Vec<Group> = Vec::new();
        for total in rows {
            let total = total?;
            let category = total.name.category.clone();
            let existing = category
                .as_ref()
                .and_then(|c| groups.iter_mut().find(|g| g.category.as_ref() == Some(c)));
            match existing {
                Some(group) => {
                    group.seconds += total.seconds;
                    group.activities.push(total);
                }
                None => groups.push(Group {
                    category,
                    seconds: total.seconds,
                    activities: vec![total],
                }),
            }
        }
        for group in &mut groups {
            group.activities.sort_by(|a, b| {
                b.seconds
                    .cmp(&a.seconds)
                    .then_with(|| a.name.name.cmp(&b.name.name))
            });
        }
        groups.sort_by(|a, b| {
            b.seconds
                .cmp(&a.seconds)
                .then_with(|| label(a).cmp(label(b)))
        });

        let (first, last): (Option<i64>, Option<i64>) = self.conn.query_row(
            "SELECT MIN(MAX(start_at, ?1)), MAX(MIN(end_at, ?2))
             FROM entries WHERE end_at > ?1 AND start_at < ?2",
            params![start, end],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(Totals {
            seconds: groups.iter().map(|g| g.seconds).sum(),
            groups,
            first: first.map(from_secs),
            last: last.map(from_secs),
        })
    }

    /// One activity's total in the range.
    pub(crate) fn activity_total(&self, id: ActivityId, range: &TimeRange) -> Result<i64> {
        Ok(self.conn.query_row(
            "SELECT IFNULL((SELECT SUM(MIN(end_at, ?3) - MAX(start_at, ?2)) FROM entries
                            WHERE activity_id = ?1 AND end_at > ?2 AND start_at < ?3), 0)
                  + IFNULL((SELECT SUM(seconds) FROM corrections
                            WHERE activity_id = ?1 AND day BETWEEN ?4 AND ?5), 0)",
            params![
                id,
                secs(range.start),
                secs(range.end),
                day_key(range.first_day),
                day_key(range.last_day)
            ],
            |row| row.get(0),
        )?)
    }
}

fn label(group: &Group) -> &str {
    group
        .category
        .as_deref()
        .unwrap_or(&group.activities[0].name.name)
}

#[cfg(test)]
mod tests;
