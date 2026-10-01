//! UTC timestamps for manifests, rows and sidecars. RFC 3339 at second precision, always with `Z`.

use std::time::{SystemTime, UNIX_EPOCH};

/// The current time, for example `2026-10-01T12:34:56Z`.
pub fn now_rfc3339() -> String {
    format_rfc3339(unix_now())
}

/// Today's UTC date, for example `2026-10-01`.
pub fn today() -> String {
    let (y, m, d) = civil_from_days(unix_now().div_euclid(86_400));
    format!("{y:04}-{m:02}-{d:02}")
}

fn unix_now() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
        Err(e) => -i64::try_from(e.duration().as_secs()).unwrap_or(i64::MAX),
    }
}

pub(crate) fn format_rfc3339(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

/// Days since 1970-01-01 to a proleptic Gregorian date (Howard Hinnant's `civil_from_days`).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_instants() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_rfc3339(1_790_812_800 + 3_723), "2026-10-01T01:02:03Z");
        assert_eq!(format_rfc3339(-1), "1969-12-31T23:59:59Z");
        assert_eq!(format_rfc3339(4_107_542_399), "2100-02-28T23:59:59Z");
    }

    #[test]
    fn now_has_the_documented_shape() {
        let s = now_rfc3339();
        assert_eq!(s.len(), 20, "{s}");
        assert!(s.ends_with('Z') && s.as_bytes()[10] == b'T');
        assert_eq!(today().len(), 10);
    }
}
