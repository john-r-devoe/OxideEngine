//! Mapping from arbitrary CSV headers to the canonical [`Bar`](crate::data::Bar) fields.
//!
//! Users pass `{"<their column>": "<canonical field>"}`. Without a schema the
//! loader normalizes headers (strip `<`/`>`, trim, lowercase) and matches them
//! against canonical names and common aliases (`date`, `vol`, ...).

use std::collections::HashMap;

use crate::error::{OxideError, OxideResult};

/// The six canonical bar fields every data source must provide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanonicalField {
    Timestamp,
    Open,
    High,
    Low,
    Close,
    Volume,
}

impl CanonicalField {
    pub const ALL: [Self; 6] = [
        Self::Timestamp,
        Self::Open,
        Self::High,
        Self::Low,
        Self::Close,
        Self::Volume,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timestamp => "timestamp",
            Self::Open => "open",
            Self::High => "high",
            Self::Low => "low",
            Self::Close => "close",
            Self::Volume => "volume",
        }
    }

    /// Parses an exact canonical name (as used in a user schema's values).
    pub fn parse(name: &str) -> OxideResult<Self> {
        Self::ALL
            .into_iter()
            .find(|f| f.as_str() == name)
            .ok_or_else(|| {
                let valid: Vec<&str> = Self::ALL.iter().map(|f| f.as_str()).collect();
                OxideError::Schema(format!(
                    "unknown canonical field '{name}'; expected one of {valid:?}"
                ))
            })
    }
}

/// Column positions of each canonical field within a CSV header row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnIndex {
    pub timestamp: usize,
    /// Optional separate time-of-day column (e.g. Stooq's `<TIME>`), merged into `timestamp`.
    pub time: Option<usize>,
    pub open: usize,
    pub high: usize,
    pub low: usize,
    pub close: usize,
    pub volume: usize,
}

/// A validated user-supplied header mapping.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColumnSchema {
    /// user column name -> canonical field
    mapping: HashMap<String, CanonicalField>,
}

impl ColumnSchema {
    /// Validates that every target is a canonical field and no field is mapped twice.
    /// Completeness (all six fields present) is checked against the real header in [`resolve`](Self::resolve).
    pub fn from_mapping<I>(pairs: I) -> OxideResult<Self>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut mapping = HashMap::new();
        let mut seen: HashMap<CanonicalField, String> = HashMap::new();
        for (column, target) in pairs {
            let field = CanonicalField::parse(target.trim())?;
            if let Some(previous) = seen.insert(field, column.clone()) {
                return Err(OxideError::Schema(format!(
                    "columns '{previous}' and '{column}' both map to '{}'",
                    field.as_str()
                )));
            }
            mapping.insert(column, field);
        }
        Ok(Self { mapping })
    }

    pub fn field_for(&self, column: &str) -> Option<CanonicalField> {
        self.mapping.get(column).copied()
    }

    /// Locates every canonical field in `headers`, erroring on any missing field.
    pub fn resolve(&self, headers: &[String]) -> OxideResult<ColumnIndex> {
        locate(headers, None, |header, field| {
            self.field_for(header.trim()) == Some(field)
        })
        .map_err(|missing| {
            OxideError::Schema(format!(
                "no column in {headers:?} is mapped to {missing:?}; \
                 the schema must map a header column to every one of these fields"
            ))
        })
    }
}

/// Header names (after [`normalize_header`]) recognized for each field without a schema.
fn aliases(field: CanonicalField) -> &'static [&'static str] {
    match field {
        CanonicalField::Timestamp => &["timestamp", "date", "datetime"],
        CanonicalField::Open => &["open"],
        CanonicalField::High => &["high"],
        CanonicalField::Low => &["low"],
        CanonicalField::Close => &["close"],
        CanonicalField::Volume => &["volume", "vol"],
    }
}

/// Normalizes a raw header for schema-less matching: strips `<`/`>`, trims, lowercases.
pub fn normalize_header(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .to_lowercase()
}

/// Resolves column positions from normalized headers when the user gave no schema.
/// A separate `time` column (Stooq's `<TIME>`) is picked up when present.
pub fn auto_detect(headers: &[String]) -> OxideResult<ColumnIndex> {
    let normalized: Vec<String> = headers.iter().map(|h| normalize_header(h)).collect();
    let time = normalized.iter().position(|h| h == "time");
    locate(&normalized, time, |header, field| {
        aliases(field).contains(&header)
    })
    .map_err(|missing| {
        OxideError::Schema(format!(
            "could not detect {missing:?} among headers {headers:?}; \
             pass a schema mapping your columns to canonical fields"
        ))
    })
}

/// Finds the first header matching each canonical field, or returns the names of
/// every field with no match.
fn locate(
    headers: &[String],
    time: Option<usize>,
    matches: impl Fn(&str, CanonicalField) -> bool,
) -> Result<ColumnIndex, Vec<&'static str>> {
    let positions = CanonicalField::ALL.map(|field| headers.iter().position(|h| matches(h, field)));
    match positions {
        [Some(timestamp), Some(open), Some(high), Some(low), Some(close), Some(volume)] => {
            Ok(ColumnIndex {
                timestamp,
                time,
                open,
                high,
                low,
                close,
                volume,
            })
        }
        _ => Err(CanonicalField::ALL
            .into_iter()
            .zip(positions)
            .filter(|(_, position)| position.is_none())
            .map(|(field, _)| field.as_str())
            .collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn parses_every_canonical_name() {
        for field in CanonicalField::ALL {
            assert_eq!(CanonicalField::parse(field.as_str()).unwrap(), field);
        }
    }

    #[test]
    fn builds_schema_from_valid_mapping() {
        let schema =
            ColumnSchema::from_mapping(pairs(&[("Date", "timestamp"), ("Vol", "volume")])).unwrap();
        assert_eq!(schema.field_for("Date"), Some(CanonicalField::Timestamp));
        assert_eq!(schema.field_for("Vol"), Some(CanonicalField::Volume));
        assert_eq!(schema.field_for("Other"), None);
    }

    #[test]
    fn rejects_unknown_canonical_target() {
        let err = ColumnSchema::from_mapping(pairs(&[("Adj", "adj_close")])).unwrap_err();
        assert!(err.to_string().contains("adj_close"));
    }

    #[test]
    fn rejects_two_columns_mapped_to_same_field() {
        let err = ColumnSchema::from_mapping(pairs(&[("Close", "close"), ("Adj Close", "close")]))
            .unwrap_err();
        assert!(err.to_string().contains("'close'"));
    }

    fn headers(items: &[&str]) -> Vec<String> {
        items.iter().map(|h| h.to_string()).collect()
    }

    fn custom_schema() -> ColumnSchema {
        ColumnSchema::from_mapping(pairs(&[
            ("Date", "timestamp"),
            ("Open", "open"),
            ("High", "high"),
            ("Low", "low"),
            ("Close", "close"),
            ("Vol", "volume"),
        ]))
        .unwrap()
    }

    #[test]
    fn normalizes_brackets_whitespace_and_case() {
        assert_eq!(normalize_header("<CLOSE>"), "close");
        assert_eq!(normalize_header("  Vol "), "vol");
        assert_eq!(normalize_header("< Date >"), "date");
    }

    #[test]
    fn auto_detects_stooq_headers_with_time_column() {
        let index = auto_detect(&headers(&[
            "<TICKER>",
            "<PER>",
            "<DATE>",
            "<TIME>",
            "<OPEN>",
            "<HIGH>",
            "<LOW>",
            "<CLOSE>",
            "<VOL>",
            "<OPENINT>",
        ]))
        .unwrap();
        let expected = ColumnIndex {
            timestamp: 2,
            time: Some(3),
            open: 4,
            high: 5,
            low: 6,
            close: 7,
            volume: 8,
        };
        assert_eq!(index, expected);
    }

    #[test]
    fn auto_detects_plain_headers_in_any_order() {
        let index = auto_detect(&headers(&[
            "Volume",
            "Close",
            "Low",
            "High",
            "Open",
            "Timestamp",
        ]))
        .unwrap();
        let expected = ColumnIndex {
            timestamp: 5,
            time: None,
            open: 4,
            high: 3,
            low: 2,
            close: 1,
            volume: 0,
        };
        assert_eq!(index, expected);
    }

    #[test]
    fn auto_detect_names_every_missing_field() {
        let err = auto_detect(&headers(&["Date", "Open", "Price"])).unwrap_err();
        let message = err.to_string();
        assert!(matches!(err, OxideError::Schema(_)));
        for missing in ["high", "low", "close", "volume"] {
            assert!(message.contains(missing), "{message}");
        }
    }

    #[test]
    fn resolves_user_schema_against_header() {
        let index = custom_schema()
            .resolve(&headers(&["Vol", "Date", "Open", "High", "Low", "Close"]))
            .unwrap();
        let expected = ColumnIndex {
            timestamp: 1,
            time: None,
            open: 2,
            high: 3,
            low: 4,
            close: 5,
            volume: 0,
        };
        assert_eq!(index, expected);
    }

    #[test]
    fn resolve_errors_when_mapped_column_is_absent_from_header() {
        let err = custom_schema()
            .resolve(&headers(&["Date", "Open", "High", "Low", "Vol"]))
            .unwrap_err();
        assert!(matches!(err, OxideError::Schema(_)));
        assert!(err.to_string().contains("close"));
    }

    #[test]
    fn resolve_errors_when_schema_does_not_cover_every_field() {
        let schema = ColumnSchema::from_mapping(pairs(&[("Date", "timestamp")])).unwrap();
        let err = schema
            .resolve(&headers(&["Date", "Open", "High", "Low", "Close", "Vol"]))
            .unwrap_err();
        assert!(err.to_string().contains("open"));
    }
}
