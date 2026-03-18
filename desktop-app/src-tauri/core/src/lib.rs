pub mod api;
pub mod idle;
pub mod models;
pub mod timer_state;
pub mod protocol;

/// Common constants used across the application.
/// Centralizing these prevents magic numbers and ensures consistency.
pub mod constants {
    /// Interval between idle detection polls (seconds)
    pub const POLL_INTERVAL_SECS: u64 = 2;

    /// Interval between API sync operations (seconds)
    pub const SYNC_INTERVAL_SECS: u64 = 30;

    /// Interval between timestamp recordings (seconds)
    pub const TIMESTAMP_INTERVAL_SECS: u64 = 5;

    /// Default idle threshold before stopping timer (seconds)
    pub const DEFAULT_IDLE_THRESHOLD_SECS: u64 = 60;

    /// Threshold for considering user active (seconds)
    pub const ACTIVE_THRESHOLD_SECS: u64 = 3;

    /// Maximum timestamps to retain per entry
    pub const MAX_TIMESTAMPS_PER_ENTRY: usize = 200;
}

/// Format seconds into a human-readable duration string.
///
/// # Examples
/// ```
/// use timez_core::format_duration;
/// assert_eq!(format_duration(45), "45 sec");
/// assert_eq!(format_duration(90), "1 min 30 sec");
/// assert_eq!(format_duration(3665), "1 hr 1 min");
/// ```
#[must_use]
pub fn format_duration(secs: i64) -> String {
    if secs < 60 {
        format!("{} sec", secs)
    } else if secs < 3600 {
        let mins = secs / 60;
        let remaining_secs = secs % 60;
        if remaining_secs > 0 {
            format!("{} min {} sec", mins, remaining_secs)
        } else {
            format!("{} min", mins)
        }
    } else {
        let hrs = secs / 3600;
        let mins = (secs % 3600) / 60;
        if mins > 0 {
            format!("{} hr {} min", hrs, mins)
        } else {
            format!("{} hr", hrs)
        }
    }
}
