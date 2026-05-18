//! Render UTC timestamps in the configured display timezone.
//!
//! Storage is always UTC (clients send RFC3339 UTC; SQLite `datetime('now')`
//! also returns UTC). Display happens in `Config::app_timezone` — defaults to
//! `Asia/Ho_Chi_Minh` so the admin UI matches the team's wall clock.

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

/// Parse a stored UTC timestamp. Accepts either RFC3339 (e.g.
/// `2026-05-18T07:30:00+00:00`, what desktop clients send) or the SQLite
/// `datetime('now')` shape (`2026-05-18 07:30:00`).
pub fn parse_utc(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
        .or_else(|| {
            NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .ok()
                .map(|n| DateTime::from_naive_utc_and_offset(n, Utc))
        })
}

/// Format a stored UTC timestamp into `YYYY-MM-DD HH:MM:SS` in `tz`. Returns
/// the original string if parsing fails so the admin UI degrades gracefully
/// instead of showing `—` for unexpected formats.
pub fn fmt_local(raw: &str, tz: Tz) -> String {
    match parse_utc(raw) {
        Some(dt) => tz.from_utc_datetime(&dt.naive_utc())
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        None => raw.to_string(),
    }
}

/// Same as [`fmt_local`] but accepts an `Option<&str>`. `None` and empty
/// strings produce the em-dash placeholder used across the admin templates.
pub fn fmt_local_opt(raw: Option<&str>, tz: Tz) -> String {
    match raw {
        None | Some("") => "—".to_string(),
        Some(s) => fmt_local(s, tz),
    }
}
