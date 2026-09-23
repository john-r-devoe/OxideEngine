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
    pub const ALL: [Self; 6] = [Self::Timestamp, Self::Open, Self::High, Self::Low, Self::Close, Self::Volume];

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
        Self::ALL.into_iter().find(|f| f.as_str() == name).ok_or_else(|| {
            let valid: Vec<&str> = Self::ALL.iter().map(|f| f.as_str()).collect();
            OxideError::Schema(format!("unknown canonical field '{name}'; expected one of {valid:?}"))
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
    pub fn resolve(&self, _headers: &[String]) -> OxideResult<ColumnIndex> {
        Err(OxideError::NotImplemented("schema header resolution (data::schema::ColumnSchema::resolve)"))
    }
}

/// Normalizes a raw header for schema-less matching: strips `<`/`>`, trims, lowercases.
pub fn normalize_header(_raw: &str) -> String {
    todo!("header normalization (data::schema::normalize_header)")
}

/// Resolves column positions from normalized headers when the user gave no schema.
pub fn auto_detect(_headers: &[String]) -> OxideResult<ColumnIndex> {
    Err(OxideError::NotImplemented("automatic header detection (data::schema::auto_detect)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn parses_every_canonical_name() {
        for field in CanonicalField::ALL {
            assert_eq!(CanonicalField::parse(field.as_str()).unwrap(), field);
        }
    }

    #[test]
    fn builds_schema_from_valid_mapping() {
        let schema = ColumnSchema::from_mapping(pairs(&[("Date", "timestamp"), ("Vol", "volume")])).unwrap();
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
        let err = ColumnSchema::from_mapping(pairs(&[("Close", "close"), ("Adj Close", "close")])).unwrap_err();
        assert!(err.to_string().contains("'close'"));
    }
}
