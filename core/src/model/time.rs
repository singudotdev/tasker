//! Timestamps as stored in task files, and ages as shown on screen.

use chrono::{DateTime, Duration, Local, NaiveDateTime, TimeZone, Timelike};

/// RFC 3339 with the local UTC offset, e.g. `2026-09-24T10:12:00+02:00`.
const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%:z";

/// Local-time formats accepted when a timestamp is typed by hand.
const HAND_TYPED_FORMATS: [&str; 4] = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%dT%H:%M"];

/// Current local time truncated to whole seconds, so files round-trip exactly.
pub fn now() -> DateTime<Local> {
    let now = Local::now();
    now.with_nanosecond(0).unwrap_or(now)
}

/// A timestamp in the stored format.
pub fn fmt_ts(dt: DateTime<Local>) -> String {
    dt.format(TIMESTAMP_FORMAT).to_string()
}

/// Accepts RFC 3339 plus the hand-typing friendly local formats above.
pub fn parse_ts(s: &str) -> Option<DateTime<Local>> {
    let s = s.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Local));
    }
    HAND_TYPED_FORMATS
        .iter()
        .find_map(|format| NaiveDateTime::parse_from_str(s, format).ok())
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
}

/// How long ago, rounded down: `0m`, `5m`, `3h`, `2d`, `6w`.
pub fn fmt_age(d: Duration) -> String {
    let minutes = d.num_minutes().max(0);
    match minutes {
        ..60 => format!("{minutes}m"),
        60..1440 => format!("{}h", minutes / 60),
        1440..20160 => format!("{}d", minutes / 1440),
        _ => format!("{}w", minutes / 10080),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hand_typed_and_stored_formats_agree() {
        let typed = parse_ts("2026-09-24 10:12").unwrap();
        assert_eq!(parse_ts(&fmt_ts(typed)), Some(typed));
        assert_eq!(parse_ts("yesterday"), None);
    }

    #[test]
    fn ages() {
        assert_eq!(fmt_age(Duration::seconds(59)), "0m");
        assert_eq!(fmt_age(Duration::minutes(-5)), "0m");
        assert_eq!(fmt_age(Duration::minutes(65)), "1h");
        assert_eq!(fmt_age(Duration::days(3)), "3d");
        assert_eq!(fmt_age(Duration::days(15)), "2w");
    }
}
