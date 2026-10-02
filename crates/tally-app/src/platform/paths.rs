//! Where tally keeps its files: `~/Library/Application Support/tally/` on macOS,
//! `%APPDATA%\tally\` on Windows, `~/.local/share/tally/` on Linux.

use std::fs::{File, TryLockError};
use std::path::PathBuf;

use anyhow::Context;
use directories::BaseDirs;

#[derive(Debug, Clone)]
pub struct Paths {
    pub data_dir: PathBuf,
    pub database: PathBuf,
    pub logs: PathBuf,
}

impl Paths {
    pub fn in_dir(data_dir: PathBuf) -> Self {
        Self {
            database: data_dir.join("tally.db"),
            logs: data_dir.join("logs"),
            data_dir,
        }
    }

    /// The OS data folder, or `$TALLY_DATA_DIR` when set (for trying tally out without
    /// touching real data). Does not create anything.
    pub fn locate() -> anyhow::Result<Self> {
        if let Some(dir) = std::env::var_os("TALLY_DATA_DIR") {
            return Ok(Self::in_dir(PathBuf::from(dir)));
        }
        let base = BaseDirs::new().context("could not find the home folder")?;
        Ok(Self::in_dir(base.data_dir().join("tally")))
    }

    /// Creates the data folder (`Store::open` does not create parent folders).
    pub fn create_data_dir(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.data_dir)
    }

    /// Takes the single-instance lock, held until the returned file is dropped.
    /// `Ok(None)` means another tally already holds it: two copies would both prompt
    /// and record the same time twice.
    pub fn lock_instance(&self) -> std::io::Result<Option<File>> {
        let file = File::create(self.data_dir.join("tally.lock"))?;
        match file.try_lock() {
            Ok(()) => Ok(Some(file)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Error(error)) => Err(error),
        }
    }

    /// The database path with the home folder written as `~`, for display.
    pub fn display_database(&self) -> String {
        let full = self.database.display().to_string();
        match BaseDirs::new() {
            Some(base) => {
                let home = base.home_dir().display().to_string();
                match full.strip_prefix(&home) {
                    Some(rest) if !home.is_empty() => format!("~{rest}"),
                    _ => full,
                }
            }
            None => full,
        }
    }
}

#[cfg(test)]
mod tests;
