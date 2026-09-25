//! Parsing of date/time cells into Unix epoch milliseconds (UTC).
//!
//! Formats to support: `YYYYMMDD` (+ optional `HHMMSS` time column, Stooq),
//! `YYYY-MM-DD`, `YYYY-MM-DD HH:MM:SS`, ISO-8601 with `T`, and integer epoch
//! seconds/milliseconds.

use crate::error::{OxideError, OxideResult};

/// Parses a date cell (and optional separate time cell) into epoch milliseconds.
pub fn parse_timestamp(_date: &str, _time: Option<&str>) -> OxideResult<i64> {
    Err(OxideError::NotImplemented(
        "timestamp parsing (data::timestamp::parse_timestamp)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const JAN_2_2024_MS: i64 = 1_704_153_600_000;
    const NINE_THIRTY_MS: i64 = (9 * 3600 + 30 * 60) * 1000;

    #[test]
    fn parses_supported_date_formats() {
        let cases = [
            ("20240102", None, JAN_2_2024_MS),
            ("20240102", Some("093000"), JAN_2_2024_MS + NINE_THIRTY_MS),
            ("2024-01-02", None, JAN_2_2024_MS),
            ("2024-01-02", Some("09:30:00"), JAN_2_2024_MS + NINE_THIRTY_MS),
            ("2024-01-02 09:30:00", None, JAN_2_2024_MS + NINE_THIRTY_MS),
            ("2024-01-02T09:30:00Z", None, JAN_2_2024_MS + NINE_THIRTY_MS),
            (" 2024-01-02 ", None, JAN_2_2024_MS),
            ("1704153600", None, JAN_2_2024_MS),
            ("1704153600000", None, JAN_2_2024_MS),
            ("1970-01-01", None, 0),
        ];
        for (date, time, expected) in cases {
            assert_eq!(parse_timestamp(date, time).unwrap(), expected, "{date} {time:?}");
        }
    }

    #[test]
    fn accepts_leap_day() {
        let feb_29 = parse_timestamp("2024-02-29", None).unwrap();
        let mar_1 = parse_timestamp("2024-03-01", None).unwrap();
        assert_eq!(mar_1 - feb_29, 86_400_000);
    }

    #[test]
    fn rejects_invalid_dates_and_times() {
        let cases = [
            ("", None),
            ("abc", None),
            ("2024-13-01", None),
            ("2023-02-29", None),
            ("2024-01-02 25:00:00", None),
            ("20240102", Some("9:30")),
            ("2024-01-02-03", None),
        ];
        for (date, time) in cases {
            let err = parse_timestamp(date, time).unwrap_err();
            assert!(matches!(err, OxideError::InvalidBar(_)), "{date} {time:?}: {err}");
        }
    }
}
