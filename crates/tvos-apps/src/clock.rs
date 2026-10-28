//! Wall-clock time for Italy (CET/CEST) without any timezone database.

use std::time::{SystemTime, UNIX_EPOCH};

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn year_of(unix: i64) -> i64 {
    let z = unix.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    yoe + era * 400 + i64::from(m <= 2)
}

/// Unix time of 01:00 UTC on the last Sunday of `month` (March and October both have 31 days).
fn last_sunday_01utc(year: i64, month: i64) -> i64 {
    let last = days_from_civil(year, month, 31);
    let weekday = (last + 4).rem_euclid(7); // 0 = Sunday
    (last - weekday) * 86_400 + 3_600
}

/// Offset from UTC in seconds: +2 h in summer time, +1 h otherwise.
pub fn italy_offset(unix: i64) -> i64 {
    let y = year_of(unix);
    if unix >= last_sunday_01utc(y, 3) && unix < last_sunday_01utc(y, 10) {
        7_200
    } else {
        3_600
    }
}

pub fn italy_hm(unix: i64) -> (u32, u32) {
    let local = unix + italy_offset(unix);
    let secs = local.rem_euclid(86_400);
    ((secs / 3_600) as u32, ((secs % 3_600) / 60) as u32)
}

/// "09:41", or "--:--" while the system date is obviously unset (before NTP on a Pi with no battery clock).
pub fn now_text() -> String {
    let unix = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
    if unix < 1_700_000_000 {
        return "--:--".into();
    }
    let (h, m) = italy_hm(unix);
    format!("{h:02}:{m:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unix(y: i64, m: i64, d: i64, h: i64, min: i64) -> i64 {
        days_from_civil(y, m, d) * 86_400 + h * 3_600 + min * 60
    }

    #[test]
    fn winter_is_utc_plus_one() {
        assert_eq!(italy_hm(unix(2026, 10, 27, 15, 0)), (16, 0)); // DST ended on 25 Oct 2026
        assert_eq!(italy_hm(unix(2026, 1, 15, 23, 30)), (0, 30)); // crosses midnight
    }

    #[test]
    fn summer_is_utc_plus_two() {
        assert_eq!(italy_hm(unix(2026, 7, 1, 12, 0)), (14, 0));
    }

    #[test]
    fn switch_moments_2026() {
        // last Sunday of March 2026 is the 29th: 01:00 UTC is the switch
        assert_eq!(italy_hm(unix(2026, 3, 29, 0, 59)), (1, 59));
        assert_eq!(italy_hm(unix(2026, 3, 29, 1, 0)), (3, 0));
        // last Sunday of October 2026 is the 25th
        assert_eq!(italy_hm(unix(2026, 10, 25, 0, 59)), (2, 59));
        assert_eq!(italy_hm(unix(2026, 10, 25, 1, 0)), (2, 0));
    }

    #[test]
    fn year_extraction() {
        assert_eq!(year_of(unix(2026, 12, 31, 23, 59)), 2026);
        assert_eq!(year_of(unix(2027, 1, 1, 0, 0)), 2027);
        assert_eq!(year_of(unix(2024, 2, 29, 12, 0)), 2024);
    }
}
