//! Shared audio utilities.

use std::time::{SystemTime, UNIX_EPOCH};

/// Monotonic-ish timestamp in milliseconds since UNIX epoch.
/// Used for event timestamps across audio components.
#[inline]
pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Generate a simple unique id based on current time in nanoseconds.
/// Sufficient for tagging short-lived audio events.
#[inline]
pub(crate) fn gen_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}
