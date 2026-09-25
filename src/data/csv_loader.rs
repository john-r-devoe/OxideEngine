//! CSV file -> [`Ticker`](crate::data::Ticker).
//!
//! 1. read the header row and resolve columns via the user's `schema`, or
//!    [`schema::auto_detect`] when none is given (either errors if a field is missing);
//! 2. parse each row into a validated [`Bar`], reporting failures as
//!    [`OxideError::MalformedRow`] with the 1-based data-row number;
//! 3. sort by timestamp and reject duplicates;
//! 4. symbol = explicit `symbol`, else [`infer_symbol`] from the file name.

use std::path::Path;

use csv::StringRecord;

use crate::data::schema::{self, ColumnIndex};
use crate::data::timestamp::parse_timestamp;
use crate::data::{Bar, ColumnSchema, Ticker};
use crate::error::{OxideError, OxideResult};

/// Loads one symbol's bars from a CSV file.
pub fn load_csv(
    path: &Path,
    symbol: Option<&str>,
    schema: Option<&ColumnSchema>,
) -> OxideResult<Ticker> {
    let io_error = |e: csv::Error| OxideError::Io {
        path: path.display().to_string(),
        message: e.to_string(),
    };
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .map_err(io_error)?;
    let headers: Vec<String> = reader
        .headers()
        .map_err(io_error)?
        .iter()
        .map(str::to_string)
        .collect();
    let columns = match schema {
        Some(schema) => schema.resolve(&headers)?,
        None => schema::auto_detect(&headers)?,
    };

    let mut bars = reader
        .records()
        .enumerate()
        .map(|(i, record)| {
            record
                .map_err(|e| e.to_string())
                .and_then(|record| parse_bar(&record, &columns))
                .map_err(|message| OxideError::MalformedRow {
                    row: i + 1,
                    message,
                })
        })
        .collect::<OxideResult<Vec<Bar>>>()?;
    bars.sort_by_key(|bar| bar.timestamp);
    if let Some(pair) = bars
        .windows(2)
        .find(|pair| pair[0].timestamp == pair[1].timestamp)
    {
        return Err(OxideError::InvalidTicker(format!(
            "duplicate timestamp {} in '{}'",
            pair[0].timestamp,
            path.display()
        )));
    }

    let symbol = match symbol {
        Some(symbol) => symbol.to_string(),
        None => infer_symbol(path)?,
    };
    Ticker::new(symbol, bars)
}

/// Builds one validated bar from a data row.
fn parse_bar(record: &StringRecord, columns: &ColumnIndex) -> Result<Bar, String> {
    // The reader rejects rows whose width differs from the header, so every index exists.
    let cell = |i: usize| record.get(i).unwrap_or_default();
    let number = |i: usize, name: &str| {
        cell(i)
            .parse::<f64>()
            .map_err(|_| format!("{name} '{}' is not a number", cell(i)))
    };
    let timestamp = parse_timestamp(cell(columns.timestamp)).map_err(|e| e.to_string())?;
    Bar::new(
        timestamp,
        number(columns.open, "open")?,
        number(columns.high, "high")?,
        number(columns.low, "low")?,
        number(columns.close, "close")?,
        number(columns.volume, "volume")?,
    )
    .map_err(|e| e.to_string())
}

/// Derives a symbol from a file name: `AAPL.us.txt` -> `AAPL`, `msft.csv` -> `MSFT`.
pub fn infer_symbol(path: &Path) -> OxideResult<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split('.').next())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .map(str::to_uppercase)
        .ok_or_else(|| {
            OxideError::InvalidTicker(format!(
                "cannot infer a symbol from '{}'; pass `symbol` explicitly",
                path.display()
            ))
        })
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
        assert!(
            matches!(err, OxideError::MalformedRow { row: 2, .. }),
            "{err}"
        );
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
        assert!(
            matches!(err, OxideError::MalformedRow { row: 1, .. }),
            "{err}"
        );
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
