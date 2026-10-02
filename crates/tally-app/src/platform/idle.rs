//! Time since the last keyboard or mouse input, polled on every tick.

use jiff::SignedDuration;
use user_idle::UserIdle;

/// OS idle time. If the OS can't tell us, report "not idle": tally keeps prompting
/// rather than wrongly marking the user away (sleep is still caught by tick gaps).
pub fn idle_for() -> SignedDuration {
    match UserIdle::get_time() {
        Ok(idle) => SignedDuration::from_secs(i64::try_from(idle.as_seconds()).unwrap_or(i64::MAX)),
        Err(error) => {
            tracing::debug!(%error, "idle time unavailable");
            SignedDuration::ZERO
        }
    }
}
