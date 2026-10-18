//! Text the views show: durations, clock times, time windows and range titles.
//! All pure: time and zone come in as parameters.

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use tally_core::duration::format_hm;
use tally_core::range::{RangeSpec, RangeUnit};

/// A duration as `6h 35m` (the Duration Rule). Negative values show as `0h 00m`.
pub fn duration(seconds: i64) -> String {
    format_hm(seconds)
}

/// A short duration for sentences: `37 min` under an hour, otherwise `1h 05m`.
pub fn short_duration(seconds: i64) -> String {
    let minutes = seconds.max(0) / 60;
    if minutes < 60 {
        format!("{minutes} min")
    } else {
        format_hm(seconds)
    }
}

/// A clock time such as `09:05`.
pub fn clock(at: Timestamp, tz: &TimeZone) -> String {
    at.to_zoned(tz.clone()).strftime("%H:%M").to_string()
}

/// A time window such as `09:00 – 09:15`.
pub fn window(start: Timestamp, end: Timestamp, tz: &TimeZone) -> String {
    format!("{} – {}", clock(start, tz), clock(end, tz))
}

/// The span covered by entries, `09:00 – 16:35`, or `Mon 09:00 – Wed 16:35` when it
/// crosses days.
pub fn span(first: Timestamp, last: Timestamp, tz: &TimeZone) -> String {
    let (a, b) = (first.to_zoned(tz.clone()), last.to_zoned(tz.clone()));
    if a.date() == b.date() {
        window(first, last, tz)
    } else {
        format!("{} – {}", a.strftime("%a %H:%M"), b.strftime("%a %H:%M"))
    }
}

/// `Monday 5 October`, with the year added when it isn't `today`'s year.
pub fn long_day(date: Date, today: Date) -> String {
    if date.year() == today.year() {
        date.strftime("%A %-d %B").to_string()
    } else {
        date.strftime("%A %-d %B %Y").to_string()
    }
}

/// The dashboard header: a title and a muted subtitle for the viewed range.
pub fn range_title(spec: RangeSpec, today: Date) -> (String, String) {
    let Ok((first, last)) = spec.days() else {
        return (String::from("All time"), String::new());
    };
    let this = |unit| RangeSpec::new(unit, today);
    let previous = |unit| this(unit).step(-1).ok();
    match spec.unit {
        RangeUnit::Day => {
            let title = if first == today {
                String::from("Today")
            } else if today.yesterday().ok() == Some(first) {
                String::from("Yesterday")
            } else {
                first.strftime("%A").to_string()
            };
            (title, long_day(first, today))
        }
        RangeUnit::Week => {
            let title = if this(RangeUnit::Week).days().ok() == Some((first, last)) {
                String::from("This week")
            } else if previous(RangeUnit::Week).and_then(|s| s.days().ok()) == Some((first, last)) {
                String::from("Last week")
            } else {
                format!("Week of {}", first.strftime("%-d %B"))
            };
            (title, day_span(first, last, today))
        }
        RangeUnit::Month => {
            let title = if this(RangeUnit::Month).days().ok() == Some((first, last)) {
                String::from("This month")
            } else {
                first.strftime("%B").to_string()
            };
            (title, first.strftime("%B %Y").to_string())
        }
        RangeUnit::Year => {
            let title = if first.year() == today.year() {
                String::from("This year")
            } else {
                first.year().to_string()
            };
            (title, first.year().to_string())
        }
        RangeUnit::All => (String::from("All time"), String::from("Everything tracked")),
    }
}

/// `5 – 11 October`, `28 September – 4 October`, or with years when they differ from today.
pub fn day_span(first: Date, last: Date, today: Date) -> String {
    let year = |d: Date| {
        if d.year() == today.year() {
            String::new()
        } else {
            format!(" {}", d.year())
        }
    };
    if first.month() == last.month() && first.year() == last.year() {
        format!(
            "{} – {}{}",
            first.day(),
            last.strftime("%-d %B"),
            year(last)
        )
    } else {
        format!(
            "{}{} – {}{}",
            first.strftime("%-d %B"),
            if first.year() == last.year() {
                String::new()
            } else {
                year(first)
            },
            last.strftime("%-d %B"),
            year(last)
        )
    }
}

/// Splits `text` into runs, marking the case-insensitive occurrences of any of
/// `needles` so the prompt can draw them in amber.
pub fn highlight(text: &str, needles: &[String]) -> Vec<(String, bool)> {
    let lower = text.to_lowercase();
    // Only highlight when lowercasing kept byte offsets (true for ASCII and most text).
    if lower.len() != text.len() {
        return vec![(text.to_owned(), false)];
    }
    let mut marked = vec![false; text.len()];
    for needle in needles.iter().filter(|n| !n.is_empty()) {
        let needle = needle.to_lowercase();
        let mut from = 0;
        while let Some(found) = lower[from..].find(&needle) {
            let start = from + found;
            marked[start..start + needle.len()]
                .iter_mut()
                .for_each(|m| *m = true);
            from = start + needle.len();
        }
    }
    let mut runs: Vec<(String, bool)> = Vec::new();
    for (index, ch) in text.char_indices() {
        let on = marked[index];
        match runs.last_mut() {
            Some((run, last)) if *last == on => run.push(ch),
            _ => runs.push((ch.to_string(), on)),
        }
    }
    runs
}

#[cfg(test)]
mod tests;
