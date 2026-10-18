use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// A typed activity such as `Work: Meeting` (category "Work") or `Lunch` (no category).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActivityName {
    pub category: Option<String>,
    pub name: String,
}

impl ActivityName {
    /// Parses user input, splitting at the first `": "`.
    pub fn parse(input: &str) -> Result<Self> {
        // Split before trimming the end, so "Work: " is an empty name, not "Work:".
        let (category, name) = match input.trim_start().split_once(": ") {
            Some((category, name)) => (Some(category.trim()), name.trim()),
            None => (None, input.trim()),
        };
        if name.is_empty() {
            return Err(Error::EmptyName);
        }
        let category = category.filter(|c| !c.is_empty()).map(str::to_owned);
        Ok(Self {
            category,
            name: name.to_owned(),
        })
    }
}

impl fmt::Display for ActivityName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.category {
            Some(category) => write!(f, "{category}: {}", self.name),
            None => f.write_str(&self.name),
        }
    }
}

#[cfg(test)]
mod tests;
