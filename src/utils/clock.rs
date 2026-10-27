use rand::Rng;
use std::time::{SystemTime, UNIX_EPOCH};

fn now_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// Civil date from days since the Unix epoch (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Splits epoch milliseconds into `(year, month, day, hour, minute, second, millis)` in UTC.
#[must_use]
pub fn fields(millis: u128) -> (i64, i64, i64, u128, u128, u128, u128) {
    let secs = millis / 1000;
    let days = i64::try_from(secs / 86_400).unwrap_or(0);
    let (y, m, d) = civil(days);
    (
        y,
        m,
        d,
        secs / 3600 % 24,
        secs / 60 % 60,
        secs % 60,
        millis % 1000,
    )
}

/// `2026-10-27T23:17:40.931Z`
#[must_use]
pub fn iso(millis: u128) -> String {
    let (y, m, d, h, mi, s, ms) = fields(millis);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}Z")
}

/// `2026102723174092576db`: the timestamp to the millisecond, then four random hex digits.
#[must_use]
pub fn id(millis: u128, suffix: u16) -> String {
    let (y, m, d, h, mi, s, ms) = fields(millis);
    format!("{y:04}{m:02}{d:02}{h:02}{mi:02}{s:02}{ms:03}{suffix:04x}")
}

#[must_use]
pub fn timestamp() -> String {
    iso(now_millis())
}

#[must_use]
pub fn new_id() -> String {
    id(now_millis(), rand::rng().random())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_the_epoch() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn formats_a_known_instant() {
        // 2026-10-27T23:17:40.931Z
        assert_eq!(iso(1_793_143_060_931), "2026-10-27T23:17:40.931Z");
        assert_eq!(id(1_793_143_060_931, 0x76db), "2026102723174093176db");
    }

    #[test]
    fn handles_leap_days() {
        // 2024-02-29T12:00:00.000Z
        assert_eq!(iso(1_709_208_000_000), "2024-02-29T12:00:00.000Z");
    }
}
