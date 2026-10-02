//! Date ranges shown on the dashboard: a day, week, month, year, or everything.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeUnit {
    Day,
    Week,
    Month,
    Year,
    All,
}

/// A range the user is looking at, identified by its unit and any date inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeSpec {
    pub unit: RangeUnit,
    pub anchor: Date,
}

/// A resolved range: UTC bounds `[start, end)` plus the local dates it covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRange {
    pub start: Timestamp,
    pub end: Timestamp,
    pub first_day: Date,
    pub last_day: Date,
}

impl RangeSpec {
    pub fn new(unit: RangeUnit, anchor: Date) -> Self {
        Self { unit, anchor }
    }

    /// Moves by `steps` units (negative is backwards). `All` does not move.
    pub fn step(self, steps: i32) -> Result<Self> {
        let anchor = match self.unit {
            RangeUnit::Day => self.anchor.checked_add(steps.days())?,
            RangeUnit::Week => self.anchor.checked_add((steps * 7).days())?,
            RangeUnit::Month => self.anchor.first_of_month().checked_add(steps.months())?,
            RangeUnit::Year => self.anchor.first_of_year().checked_add(steps.years())?,
            RangeUnit::All => self.anchor,
        };
        Ok(Self { anchor, ..self })
    }

    /// First and last local date in the range, inclusive. Weeks start on Monday.
    pub fn days(self) -> Result<(Date, Date)> {
        Ok(match self.unit {
            RangeUnit::Day => (self.anchor, self.anchor),
            RangeUnit::Week => {
                let back = i32::from(self.anchor.weekday().to_monday_zero_offset());
                let first = self.anchor.checked_sub(back.days())?;
                (first, first.checked_add(6.days())?)
            }
            RangeUnit::Month => (self.anchor.first_of_month(), self.anchor.last_of_month()),
            RangeUnit::Year => (self.anchor.first_of_year(), self.anchor.last_of_year()),
            RangeUnit::All => (Date::MIN, Date::MAX),
        })
    }

    pub fn resolve(self, tz: &TimeZone) -> Result<TimeRange> {
        let (first_day, last_day) = self.days()?;
        if self.unit == RangeUnit::All {
            return Ok(TimeRange {
                start: Timestamp::MIN,
                end: Timestamp::MAX,
                first_day,
                last_day,
            });
        }
        let start = first_day.to_zoned(tz.clone())?.timestamp();
        let end = last_day
            .checked_add(1.day())?
            .to_zoned(tz.clone())?
            .timestamp();
        Ok(TimeRange {
            start,
            end,
            first_day,
            last_day,
        })
    }

    /// The day an added correction is filed under: the viewed day, or for longer ranges
    /// today when today is inside the range, otherwise the range's last day.
    pub fn correction_day(self, today: Date) -> Result<Date> {
        let (first, last) = self.days()?;
        Ok(if (first..=last).contains(&today) {
            today
        } else {
            last
        })
    }
}

#[cfg(test)]
mod tests;
