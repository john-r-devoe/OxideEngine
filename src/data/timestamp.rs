//! Parsing of date/time cells into Unix epoch milliseconds (UTC).
//!
//! Formats to support: `YYYYMMDD` (+ optional `HHMMSS` time column, Stooq),
//! `YYYY-MM-DD`, `YYYY-MM-DD HH:MM:SS`, ISO-8601 with `T`, and integer epoch
//! seconds/milliseconds.

use crate::error::{OxideError, OxideResult};

const MS_PER_SECOND: i64 = 1_000;
const MS_PER_DAY: i64 = 86_400_000;
/// Integer timestamps below this are epoch seconds, at or above it epoch milliseconds
/// (1e11 s is the year 5138; 1e11 ms is March 1973).
const EPOCH_MS_THRESHOLD: i64 = 100_000_000_000;

/// Parses a date cell (and optional separate time cell) into epoch milliseconds.
///
/// An all-digit cell of exactly 8 characters is always read as `YYYYMMDD`, never
/// as epoch seconds.
pub fn parse_timestamp(date: &str, time: Option<&str>) -> OxideResult<i64> {
    let parsed = match time {
        Some(time) => parse_datetime(date.trim()).zip(parse_time(time.trim())),
        None => parse_datetime(date.trim()).map(|ms| (ms, 0)),
    };
    parsed
        .map(|(date_ms, time_ms)| date_ms + time_ms)
        .ok_or_else(|| {
            let shown = time.map_or(date.to_string(), |t| format!("{date} {t}"));
            OxideError::InvalidBar(format!("unrecognized timestamp '{shown}'"))
        })
}

/// Epoch ms from `YYYYMMDD`, integer epoch s/ms, or `YYYY-MM-DD[( |T)HH:MM:SS[Z]]`.
fn parse_datetime(s: &str) -> Option<i64> {
    if s.len() == 8 {
        if let Some(ymd) = digits(s) {
            return days_from_ymd(ymd / 10_000, ymd / 100 % 100, ymd % 100).map(|d| d * MS_PER_DAY);
        }
    }
    if let Some(epoch) = digits(s) {
        return Some(if epoch < EPOCH_MS_THRESHOLD {
            epoch * MS_PER_SECOND
        } else {
            epoch
        });
    }
    let (date, time) = match s.split_once([' ', 'T']) {
        Some((date, time)) => (date, Some(time)),
        None => (s, None),
    };
    let mut parts = date.split('-');
    let (y, m, d) = (parts.next()?, parts.next()?, parts.next()?);
    // A 4-digit year also keeps the day arithmetic far from i64 overflow.
    if parts.next().is_some() || y.len() != 4 {
        return None;
    }
    let date_ms = days_from_ymd(digits(y)?, digits(m)?, digits(d)?)? * MS_PER_DAY;
    let time_ms = time.map_or(Some(0), parse_time)?;
    Some(date_ms + time_ms)
}

/// Milliseconds since midnight from `HHMMSS` or `HH:MM:SS` (optional trailing `Z`).
fn parse_time(s: &str) -> Option<i64> {
    let hhmmss: String = s
        .trim_end_matches('Z')
        .chars()
        .filter(|&c| c != ':')
        .collect();
    if hhmmss.len() != 6 {
        return None;
    }
    let value = digits(&hhmmss)?;
    let (h, m, sec) = (value / 10_000, value / 100 % 100, value % 100);
    (h < 24 && m < 60 && sec < 60).then_some((h * 3600 + m * 60 + sec) * MS_PER_SECOND)
}

/// Parses a non-empty run of ASCII digits (no sign).
fn digits(s: &str) -> Option<i64> {
    let is_digits = !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    is_digits.then(|| s.parse().ok()).flatten()
}

/// Days since 1970-01-01 for a valid Gregorian date (Howard Hinnant's `days_from_civil`).
fn days_from_ymd(year: i64, month: i64, day: i64) -> Option<i64> {
    let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_len = match month {
        2 if is_leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1..=12 => 31,
        _ => return None,
    };
    if !(1..=month_len).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let year_of_era = y - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    const JAN_2_2024_MS: i64 = 1_704_153_600_000;
    const NINE_THIRTY_MS: i64 = (9 * 3600 + 30 * 60) * 1000;

    #[test]
    fn parses_supported_date_formats() {
        let cases = [
            ("20240102", JAN_2_2024_MS),
            ("2024-01-02", JAN_2_2024_MS),
            ("2024-01-02 09:30:00", JAN_2_2024_MS + NINE_THIRTY_MS),
            ("2024-01-02T09:30:00", JAN_2_2024_MS + NINE_THIRTY_MS),
            (" 2024-01-02 ", JAN_2_2024_MS),
            ("1704153600", JAN_2_2024_MS),
            ("1704153600000", JAN_2_2024_MS),
            ("1970-01-01", 0),
        ];
        for (cell, expected) in cases {
            assert_eq!(parse_timestamp(cell).unwrap(), expected, "{cell}");
        }
    }

    #[test]
    fn accepts_leap_day() {
        let feb_29 = parse_timestamp("2024-02-29").unwrap();
        let mar_1 = parse_timestamp("2024-03-01").unwrap();
        assert_eq!(mar_1 - feb_29, 86_400_000);
    }

    #[test]
    fn rejects_invalid_dates_and_times() {
        let cases = [
            "",
            "abc",
            "2024-13-01",
            "2023-02-29",
            "2024-01-02 25:00:00",
            "2024-01-02 093000",
            "2024-01-02 9:30:00",
            "2024-01-02-03",
            "99999999999999999-01-01",
        ];
        for cell in cases {
            let err = parse_timestamp(cell).unwrap_err();
            assert!(matches!(err, OxideError::InvalidBar(_)), "{cell}: {err}");
        }
    }

    #[test]
    fn rejects_timezone_designators() {
        for cell in ["2024-01-02T09:30:00Z", "2024-01-02T09:30:00+02:00", "2024-01-02 09:30:00-05:00"] {
            assert!(parse_timestamp(cell).is_err(), "{cell}");
        }
    }
}
