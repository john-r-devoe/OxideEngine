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
