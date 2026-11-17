//! Creating, listing, renaming, merging and archiving activities.

use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension, params};

use super::{ActivityId, Store};
use crate::{ActivityName, Error, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRow {
    pub id: ActivityId,
    pub name: ActivityName,
    pub archived: bool,
    /// All recorded time, entries plus corrections.
    pub total_seconds: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rename {
    Done,
    /// Another activity already has that name; offer to merge into it instead.
    Conflict(ActivityId),
}

/// Finds or creates the activity (and its category). Un-archives it if needed.
pub(super) fn ensure(conn: &Connection, name: &ActivityName) -> Result<ActivityId> {
    let category = name
        .category
        .as_deref()
        .map(|c| ensure_category(conn, c))
        .transpose()?;
    let existing: Option<ActivityId> = conn
        .query_row(
            "SELECT id FROM activities WHERE category_id IS ?1 AND name = ?2",
            params![category, name.name],
            |row| row.get(0),
        )
        .optional()?;
    match existing {
        Some(id) => {
            conn.execute(
                "UPDATE activities SET archived_at = NULL WHERE id = ?1",
                [id],
            )?;
            Ok(id)
        }
        None => {
            conn.execute(
                "INSERT INTO activities (category_id, name) VALUES (?1, ?2)",
                params![category, name.name],
            )?;
            Ok(conn.last_insert_rowid())
        }
    }
}

fn ensure_category(conn: &Connection, name: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO categories (name) VALUES (?1) ON CONFLICT (name) DO NOTHING",
        [name],
    )?;
    Ok(
        conn.query_row("SELECT id FROM categories WHERE name = ?1", [name], |row| {
            row.get(0)
        })?,
    )
}

fn find(conn: &Connection, name: &ActivityName) -> Result<Option<ActivityId>> {
    Ok(conn
        .query_row(
            "SELECT a.id FROM activities a LEFT JOIN categories c ON c.id = a.category_id
             WHERE c.name IS ?1 AND a.name = ?2",
            params![name.category, name.name],
            |row| row.get(0),
        )
        .optional()?)
}

fn require(conn: &Connection, id: ActivityId) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM activities WHERE id = ?1)",
        [id],
        |row| row.get(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(Error::UnknownActivity(id))
    }
}

/// Moves `from`'s entries and corrections to `into`, then deletes `from`.
fn fold_into(conn: &Connection, from: ActivityId, into: ActivityId) -> Result<()> {
    conn.execute(
        "UPDATE entries SET activity_id = ?1 WHERE activity_id = ?2",
        [into, from],
    )?;
    conn.execute(
        "UPDATE corrections SET activity_id = ?1 WHERE activity_id = ?2",
        [into, from],
    )?;
    conn.execute("DELETE FROM activities WHERE id = ?1", [from])?;
    Ok(())
}

fn delete_empty_categories(conn: &Connection) -> Result<()> {
    conn.execute(
        "DELETE FROM categories WHERE id NOT IN
         (SELECT category_id FROM activities WHERE category_id IS NOT NULL)",
        [],
    )?;
    Ok(())
}

pub(super) fn name_of(
    row: &rusqlite::Row<'_>,
    category_col: usize,
    name_col: usize,
) -> rusqlite::Result<ActivityName> {
    Ok(ActivityName {
        category: row.get(category_col)?,
        name: row.get(name_col)?,
    })
}

impl Store {
    /// Finds or creates an activity by name.
    pub fn ensure_activity(&mut self, name: &ActivityName) -> Result<ActivityId> {
        ensure(&self.conn, name)
    }

    /// Non-archived activities for the prompt, most recently used first.
    pub fn suggestions(&self, limit: usize) -> Result<Vec<ActivityName>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.name, a.name FROM activities a
             LEFT JOIN categories c ON c.id = a.category_id
             LEFT JOIN entries e ON e.activity_id = a.id
             WHERE a.archived_at IS NULL
             GROUP BY a.id
             ORDER BY MAX(e.end_at) DESC NULLS LAST, a.id DESC
             LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |row| name_of(row, 0, 1))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every activity, archived ones included, sorted by category then name.
    pub fn activities(&self) -> Result<Vec<ActivityRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT a.id, c.name, a.name, a.archived_at IS NOT NULL,
                    IFNULL((SELECT SUM(end_at - start_at) FROM entries WHERE activity_id = a.id), 0)
                  + IFNULL((SELECT SUM(seconds) FROM corrections WHERE activity_id = a.id), 0)
             FROM activities a LEFT JOIN categories c ON c.id = a.category_id
             ORDER BY c.name IS NOT NULL, c.name, a.name",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ActivityRow {
                id: row.get(0)?,
                name: name_of(row, 1, 2)?,
                archived: row.get(3)?,
                total_seconds: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Renames an activity, which also moves it between categories.
    pub fn rename(&mut self, id: ActivityId, to: &ActivityName) -> Result<Rename> {
        let tx = self.conn.transaction()?;
        require(&tx, id)?;
        match find(&tx, to)? {
            Some(existing) if existing == id => return Ok(Rename::Done),
            Some(existing) => return Ok(Rename::Conflict(existing)),
            None => {}
        }
        let category = to
            .category
            .as_deref()
            .map(|c| ensure_category(&tx, c))
            .transpose()?;
        tx.execute(
            "UPDATE activities SET category_id = ?1, name = ?2 WHERE id = ?3",
            params![category, to.name, id],
        )?;
        delete_empty_categories(&tx)?;
        tx.commit()?;
        Ok(Rename::Done)
    }

    /// Moves all of `from`'s time to `into`, then deletes `from`.
    pub fn merge(&mut self, from: ActivityId, into: ActivityId) -> Result<()> {
        if from == into {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        require(&tx, from)?;
        require(&tx, into)?;
        fold_into(&tx, from, into)?;
        delete_empty_categories(&tx)?;
        tx.commit()?;
        Ok(())
    }

    /// Moves every activity in category `from` to category `to`. An activity whose
    /// name already exists in `to` is merged into it. Unknown `from` does nothing.
    pub fn rename_category(&mut self, from: &str, to: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        let Some(from_id) = tx
            .query_row("SELECT id FROM categories WHERE name = ?1", [from], |row| {
                row.get::<_, i64>(0)
            })
            .optional()?
        else {
            return Ok(());
        };
        let to_id = ensure_category(&tx, to)?;
        if to_id == from_id {
            // Only the spelling changed.
            tx.execute(
                "UPDATE categories SET name = ?1 WHERE id = ?2",
                params![to, from_id],
            )?;
        } else {
            let moving: Vec<(ActivityId, String)> = tx
                .prepare("SELECT id, name FROM activities WHERE category_id = ?1")?
                .query_map([from_id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<_>>()?;
            for (id, name) in moving {
                let target = ActivityName {
                    category: Some(to.to_owned()),
                    name,
                };
                match find(&tx, &target)? {
                    Some(into) => fold_into(&tx, id, into)?,
                    None => {
                        tx.execute(
                            "UPDATE activities SET category_id = ?1 WHERE id = ?2",
                            [to_id, id],
                        )?;
                    }
                }
            }
            delete_empty_categories(&tx)?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_archived(&mut self, id: ActivityId, archived: bool, now: Timestamp) -> Result<()> {
        require(&self.conn, id)?;
        let value = archived.then(|| now.as_second());
        self.conn.execute(
            "UPDATE activities SET archived_at = ?1 WHERE id = ?2",
            params![value, id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
