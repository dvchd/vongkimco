//! Render UTC timestamps for the admin UI.
//!
//! Storage is always UTC (clients send RFC3339 UTC; SQLite `datetime('now')`
//! also returns UTC). The admin pages now send each timestamp to the browser
//! as a pair: the canonical RFC3339 UTC string (for `Intl.DateTimeFormat` in
//! whichever timezone the viewer's browser is in) **and** a pre-rendered
//! server-side fallback in `Config::app_timezone` (default `Asia/Ho_Chi_Minh`)
//! for the no-JS / parse-failure case.

use chrono::{DateTime, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Serialize;

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
        Some(dt) => tz
            .from_utc_datetime(&dt.naive_utc())
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

/// Normalize a stored timestamp to canonical RFC3339 UTC (e.g.
/// `2026-05-19T07:30:00Z`) so the browser-side script can reformat it in the
/// viewer's timezone. Returns an empty string if parsing fails; callers should
/// treat an empty `utc` as "no client-side reformat — show the fallback".
pub fn to_utc_iso(raw: &str) -> String {
    parse_utc(raw)
        .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_default()
}

/// Same as [`to_utc_iso`] but accepts an `Option<&str>`.
pub fn to_utc_iso_opt(raw: Option<&str>) -> String {
    match raw {
        None | Some("") => String::new(),
        Some(s) => to_utc_iso(s),
    }
}

/// A timestamp cell ready for the admin templates: a canonical UTC string the
/// browser script reformats in the viewer's timezone, plus a server-rendered
/// fallback (in [`Config::app_timezone`](crate::state::Config::app_timezone))
/// that shows when JS is disabled or the UTC string can't be parsed.
///
/// Templates render this as
/// `<time data-utc="{{ cell.utc }}">{{ cell.local }}</time>` and the script in
/// `base.html` replaces the text content with the browser-localized version.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TsCell {
    pub utc: String,
    pub local: String,
}

impl TsCell {
    pub fn new(raw: &str, tz: Tz) -> Self {
        Self {
            utc: to_utc_iso(raw),
            local: fmt_local(raw, tz),
        }
    }

    pub fn opt(raw: Option<&str>, tz: Tz) -> Self {
        Self {
            utc: to_utc_iso_opt(raw),
            local: fmt_local_opt(raw, tz),
        }
    }
}
