//! CSV file -> [`Ticker`](crate::data::Ticker).
//!
//! Pipeline to implement:
//! 1. open `path` with the `csv` crate, mapping I/O failures to [`OxideError::Io`];
//! 2. resolve columns from the header row via `schema` or `schema::auto_detect`;
//! 3. per row: `timestamp::parse_timestamp` + parse OHLCV, then `Bar::new` (OHLC
//!    validation); map any failure to [`OxideError::MalformedRow`] with the 1-based
//!    data-row number;
//! 4. sort by timestamp and reject duplicate timestamps;
//! 5. symbol = explicit `symbol`, else [`infer_symbol`] from the file name.

use std::path::Path;

use crate::data::{ColumnSchema, Ticker};
use crate::error::{OxideError, OxideResult};

/// Loads one symbol's bars from a CSV file.
pub fn load_csv(
    _path: &Path,
    _symbol: Option<&str>,
    _schema: Option<&ColumnSchema>,
) -> OxideResult<Ticker> {
    Err(OxideError::NotImplemented(
        "CSV parsing (data::csv_loader::load_csv)",
    ))
}

/// Derives a symbol from a file name: `AAPL.us.txt` -> `AAPL`, `msft.csv` -> `MSFT`.
pub fn infer_symbol(_path: &Path) -> OxideResult<String> {
    Err(OxideError::NotImplemented(
        "symbol inference (data::csv_loader::infer_symbol)",
    ))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::data::Bar;

    const JAN_2_2024_MS: i64 = 1_704_153_600_000;
    const DAY_MS: i64 = 86_400_000;

    /// Writes `contents` to a per-test file under the system temp dir.
    fn write_csv(test_name: &str, file_name: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("oxide_csv_loader_{}", std::process::id()))
            .join(test_name);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(file_name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn custom_schema() -> ColumnSchema {
        let pairs = [
            ("Date", "timestamp"),
            ("Open", "open"),
            ("High", "high"),
            ("Low", "low"),
            ("Close", "close"),
            ("Vol", "volume"),
        ];
        ColumnSchema::from_mapping(pairs.map(|(a, b)| (a.to_string(), b.to_string()))).unwrap()
    }

    #[test]
    fn infers_symbol_from_file_name() {
        assert_eq!(infer_symbol(Path::new("data/AAPL.us.txt")).unwrap(), "AAPL");
        assert_eq!(infer_symbol(Path::new("msft.csv")).unwrap(), "MSFT");
        assert_eq!(infer_symbol(Path::new("spy")).unwrap(), "SPY");
    }

    #[test]
    fn rejects_file_name_without_a_stem() {
        let err = infer_symbol(Path::new(".csv")).unwrap_err();
        assert!(matches!(err, OxideError::InvalidTicker(_)));
    }

    #[test]
    fn loads_stooq_file_without_schema() {
        let path = write_csv(
            "stooq",
            "AAPL.us.txt",
            "<TICKER>,<PER>,<DATE>,<TIME>,<OPEN>,<HIGH>,<LOW>,<CLOSE>,<VOL>,<OPENINT>\n\
             AAPL.US,D,20240102,000000,100,102,99,101,1000,0\n\
             AAPL.US,D,20240103,000000,101,103,100,102,1100,0\n",
        );
        let ticker = load_csv(&path, None, None).unwrap();
        assert_eq!(ticker.symbol(), "AAPL");
        assert_eq!(
            ticker.bars()[1],
            Bar::new(JAN_2_2024_MS + DAY_MS, 101.0, 103.0, 100.0, 102.0, 1100.0).unwrap()
        );
    }

    #[test]
    fn loads_custom_headers_through_schema_with_explicit_symbol() {
        let path = write_csv(
            "schema",
            "data.csv",
            "Date,Open,High,Low,Close,Vol\n2024-01-02,10,11,9.5,10.5,1000\n",
        );
        let ticker = load_csv(&path, Some("MSFT"), Some(&custom_schema())).unwrap();
        assert_eq!(ticker.symbol(), "MSFT");
        assert_eq!(
            ticker.bars(),
            &[Bar::new(JAN_2_2024_MS, 10.0, 11.0, 9.5, 10.5, 1000.0).unwrap()]
        );
    }

    #[test]
    fn sorts_rows_by_timestamp() {
        let path = write_csv(
            "unsorted",
            "x.csv",
            "date,open,high,low,close,volume\n\
             2024-01-03,1,1,1,1,0\n\
             2024-01-02,1,1,1,1,0\n",
        );
        let ticker = load_csv(&path, None, None).unwrap();
        let timestamps: Vec<i64> = ticker.bars().iter().map(|b| b.timestamp).collect();
        assert_eq!(timestamps, [JAN_2_2024_MS, JAN_2_2024_MS + DAY_MS]);
    }

    #[test]
    fn rejects_duplicate_timestamps() {
        let path = write_csv(
            "duplicate",
            "x.csv",
            "date,open,high,low,close,volume\n\
             2024-01-02,1,1,1,1,0\n\
             2024-01-02,1,1,1,1,0\n",
        );
        let err = load_csv(&path, None, None).unwrap_err();
        assert!(err.to_string().contains("duplicate"), "{err}");
    }

    #[test]
    fn reports_malformed_row_with_one_based_row_number() {
        let path = write_csv(
            "malformed",
            "x.csv",
            "date,open,high,low,close,volume\n\
             2024-01-02,1,1,1,1,0\n\
             2024-01-03,abc,1,1,1,0\n",
        );
        let err = load_csv(&path, None, None).unwrap_err();
        assert!(matches!(err, OxideError::MalformedRow { row: 2, .. }), "{err}");
        assert!(err.to_string().contains("abc"), "{err}");
    }

    #[test]
    fn reports_ohlc_violation_as_malformed_row() {
        let path = write_csv(
            "ohlc",
            "x.csv",
            "date,open,high,low,close,volume\n2024-01-02,10,9,11,10,0\n",
        );
        let err = load_csv(&path, None, None).unwrap_err();
        assert!(matches!(err, OxideError::MalformedRow { row: 1, .. }), "{err}");
        assert!(err.to_string().contains("high"), "{err}");
    }

    #[test]
    fn errors_when_headers_cannot_be_auto_detected() {
        let path = write_csv("undetectable", "x.csv", "a,b,c\n1,2,3\n");
        let err = load_csv(&path, None, None).unwrap_err();
        assert!(matches!(err, OxideError::Schema(_)), "{err}");
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let err = load_csv(Path::new("definitely/not/here.csv"), None, None).unwrap_err();
        assert!(matches!(err, OxideError::Io { .. }), "{err}");
    }
}
