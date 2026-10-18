//! Parsing and formatting of the durations typed into the dashboard.

use crate::{Error, Result};

/// What the user asked for when editing a total.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurationEdit {
    /// Unsigned input: make the total exactly this many seconds.
    Set(i64),
    /// `+X`: add this many seconds.
    Add(i64),
    /// `-X`: remove this many seconds.
    Subtract(i64),
}

/// Parses `1h 30`, `1h30m`, `90`, `45m`, `3h`, `0`, `-30`, `+15`.
/// A bare number, or a number after hours, is minutes.
pub fn parse_edit(input: &str) -> Result<DurationEdit> {
    let trimmed = input.trim();
    let invalid = || Error::InvalidDuration(input.to_owned());
    let (sign, rest) = match trimmed.chars().next() {
        Some('+') => (Some('+'), &trimmed[1..]),
        Some('-') => (Some('-'), &trimmed[1..]),
        _ => (None, trimmed),
    };
    let seconds = parse_amount(rest).ok_or_else(invalid)?;
    match sign {
        Some('+') if seconds > 0 => Ok(DurationEdit::Add(seconds)),
        Some('-') if seconds > 0 => Ok(DurationEdit::Subtract(seconds)),
        Some(_) => Err(invalid()),
        None => Ok(DurationEdit::Set(seconds)),
    }
}

/// Parses an unsigned amount into seconds.
pub fn parse_amount(input: &str) -> Option<i64> {
    let compact: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let compact = compact.to_ascii_lowercase();
    if compact.is_empty() {
        return None;
    }
    let (hours, minutes) = match compact.split_once('h') {
        Some((hours, minutes)) => (Some(hours), minutes),
        None => (None, compact.as_str()),
    };
    let hours: i64 = match hours {
        Some(h) => h.parse().ok()?,
        None => 0,
    };
    let minutes = minutes.strip_suffix('m').unwrap_or(minutes);
    let minutes: i64 = if minutes.is_empty() {
        if hours == 0 && compact.ends_with('h') && compact != "0h" {
            return None;
        }
        0
    } else {
        minutes.parse().ok()?
    };
    if hours < 0 || minutes < 0 || (hours > 0 && minutes >= 60) {
        return None;
    }
    hours
        .checked_mul(3600)?
        .checked_add(minutes.checked_mul(60)?)
}

/// Formats seconds as `3h 05m`, rounding down to whole minutes.
pub fn format_hm(seconds: i64) -> String {
    let minutes = seconds.max(0) / 60;
    format!("{}h {:02}m", minutes / 60, minutes % 60)
}

#[cfg(test)]
mod tests;
