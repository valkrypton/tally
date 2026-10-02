//! The only place in the app that reads the clock or the system time zone.
//! Everything else takes `now` / `today` / `tz` as parameters so it can be tested.

use std::time::Instant;

use jiff::Timestamp;
use jiff::tz::TimeZone;

pub fn now() -> Timestamp {
    Timestamp::now()
}

/// Monotonic time, for debouncing UI events (never stored).
pub fn instant() -> Instant {
    Instant::now()
}

/// The system time zone, read once at startup and used for both the scheduler's
/// `Config` and every `RangeSpec::resolve`.
pub fn system_tz() -> TimeZone {
    TimeZone::system()
}
