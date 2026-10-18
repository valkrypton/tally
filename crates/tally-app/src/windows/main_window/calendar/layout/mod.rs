//! Pure calendar logic: entries to blocks, visible hours, slots, colours and the month grid.

use iced::Color;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, ToSpan};
use tally_core::ActivityName;
use tally_core::settings::WeeklySchedule;
use tally_core::store::{ActivityId, EntryRow, Totals};

use crate::theme;

/// Entries of one activity closer than this are drawn as one block.
pub const MERGE_GAP: SignedDuration = SignedDuration::from_secs(60);
pub const DEFAULT_HOURS: (i8, i8) = (9, 18);

const PALETTE: [Color; 4] = [
    theme::LAMP_AMBER,
    Color::from_rgb8(0x6E, 0xA0, 0xFF),
    Color::from_rgb8(0xB7, 0x94, 0xF6),
    Color::from_rgb8(0x5C, 0xC8, 0xE0),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub activity: ActivityId,
    pub name: ActivityName,
    pub start: Timestamp,
    pub end: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    Block(Block),
    /// Working time that is over but not logged.
    Gap {
        start: Timestamp,
        end: Timestamp,
    },
    Empty {
        start: Timestamp,
        end: Timestamp,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaySummary {
    pub seconds: i64,
    pub parts: Vec<(Option<String>, i64)>,
    pub top: Option<String>,
}

/// Entries overlapping `day`, clipped to it, with same-activity neighbours joined.
pub fn blocks(entries: &[EntryRow], day: Date, tz: &TimeZone) -> Vec<Block> {
    let (from, to) = window(day, (0, 24), tz);
    let mut clipped: Vec<Block> = entries
        .iter()
        .filter(|e| e.end > from && e.start < to)
        .map(|e| Block {
            activity: e.activity,
            name: e.name.clone(),
            start: e.start.max(from),
            end: e.end.min(to),
        })
        .collect();
    clipped.sort_by_key(|b| b.start);
    let mut out: Vec<Block> = Vec::new();
    for block in clipped {
        match out.last_mut() {
            Some(last)
                if last.activity == block.activity
                    && block.start.duration_since(last.end) <= MERGE_GAP =>
            {
                last.end = last.end.max(block.end);
            }
            _ => out.push(block),
        }
    }
    out
}

/// `[first, last)` local hours covering every working period and block, or `DEFAULT_HOURS`.
pub fn visible_hours(
    days: &[(Date, Vec<Block>)],
    schedule: &WeeklySchedule,
    tz: &TimeZone,
) -> (i8, i8) {
    // Floor of the start hour, ceil of the end hour; an end on a later date is 24.
    let span = |start: Timestamp, end: Timestamp| {
        let (s, e) = (start.to_zoned(tz.clone()), end.to_zoned(tz.clone()));
        let last = if e.date() > s.date() {
            24
        } else {
            e.hour() + i8::from(e.minute() > 0 || e.second() > 0 || e.subsec_nanosecond() > 0)
        };
        (s.hour(), last)
    };
    let spans = days.iter().flat_map(|(day, blocks)| {
        schedule
            .period_on(*day, tz)
            .into_iter()
            .chain(blocks.iter().map(|b| (b.start, b.end)))
            .map(|(start, end)| span(start, end))
    });
    spans
        .reduce(|(a, b), (c, d)| (a.min(c), b.max(d)))
        .map_or(DEFAULT_HOURS, |(a, b)| (a.clamp(0, 24), b.clamp(0, 24)))
}

/// `day` from `hours.0:00` to `hours.1:00` local time; 24 is the next midnight.
pub fn window(day: Date, hours: (i8, i8), tz: &TimeZone) -> (Timestamp, Timestamp) {
    let at = |hour: i8| {
        let (date, hour) = if hour >= 24 {
            (day.tomorrow().expect("date in range"), 0)
        } else {
            (day, hour)
        };
        date.at(hour, 0, 0, 0)
            .to_zoned(tz.clone())
            .expect("date in range")
            .timestamp()
    };
    (at(hours.0), at(hours.1))
}

/// Covers `window` exactly. Uncovered time inside `working` and before `logged_until` is a `Gap`.
pub fn slots(
    blocks: &[Block],
    window: (Timestamp, Timestamp),
    working: Option<(Timestamp, Timestamp)>,
    logged_until: Timestamp,
) -> Vec<Slot> {
    let (from, to) = window;
    let mut out: Vec<Slot> = Vec::new();
    // Uncovered time in [a, b): cut at the working bounds and `logged_until`, then classify.
    let fill = |out: &mut Vec<Slot>, a: Timestamp, b: Timestamp| {
        let mut cuts = vec![a, b, logged_until];
        cuts.extend(working.iter().flat_map(|w| [w.0, w.1]));
        cuts.retain(|c| (a..=b).contains(c));
        cuts.sort();
        cuts.dedup();
        for pair in cuts.windows(2) {
            let (start, end) = (pair[0], pair[1]);
            let gap = end <= logged_until && working.is_some_and(|w| w.0 <= start && end <= w.1);
            match (out.last_mut(), gap) {
                (Some(Slot::Gap { end: e, .. }), true)
                | (Some(Slot::Empty { end: e, .. }), false) => *e = end,
                (_, true) => out.push(Slot::Gap { start, end }),
                (_, false) => out.push(Slot::Empty { start, end }),
            }
        }
    };
    let mut cursor = from;
    for block in blocks {
        let (start, end) = (block.start.max(cursor), block.end.min(to));
        if start >= end {
            continue;
        }
        fill(&mut out, cursor, start);
        out.push(Slot::Block(Block {
            start,
            end,
            ..block.clone()
        }));
        cursor = end;
    }
    fill(&mut out, cursor, to);
    out
}

/// Whole minutes, at least 1.
pub fn portion(start: Timestamp, end: Timestamp) -> u16 {
    let minutes = (end.duration_since(start).as_secs() + 30) / 60;
    u16::try_from(minutes.max(1)).unwrap_or(u16::MAX)
}

pub fn category_color(category: Option<&str>) -> Color {
    let Some(name) = category else {
        return theme::TEXT_MUTED;
    };
    // FNV-1a over the lowercased name.
    let hash = name
        .to_lowercase()
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
        });
    // ponytail: hashed colours can collide; give categories a stored colour if users ask
    PALETTE[(hash % 4) as usize]
}

pub fn summary(totals: &Totals) -> DaySummary {
    DaySummary {
        seconds: totals.seconds,
        parts: totals
            .groups
            .iter()
            .map(|g| (g.category.clone(), g.seconds))
            .collect(),
        top: totals
            .groups
            .iter()
            .flat_map(|g| &g.activities)
            .max_by_key(|a| a.seconds)
            .map(|a| a.name.name.clone()),
    }
}

/// Whole weeks, Monday to Sunday, covering `month`: 35 or 42 dates.
pub fn month_grid(month: Date) -> Vec<Date> {
    let (first, last) = (month.first_of_month(), month.last_of_month());
    let start = first - i64::from(first.weekday().to_monday_zero_offset()).days();
    let end = last + i64::from(6 - last.weekday().to_monday_zero_offset()).days();
    start.series(1.day()).take_while(|d| *d <= end).collect()
}

#[cfg(test)]
mod tests;
