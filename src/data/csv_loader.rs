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
pub fn load_csv(_path: &Path, _symbol: Option<&str>, _schema: Option<&ColumnSchema>) -> OxideResult<Ticker> {
    Err(OxideError::NotImplemented("CSV parsing (data::csv_loader::load_csv)"))
}

/// Derives a symbol from a file name: `AAPL.us.txt` -> `AAPL`, `msft.csv` -> `MSFT`.
pub fn infer_symbol(_path: &Path) -> OxideResult<String> {
    Err(OxideError::NotImplemented("symbol inference (data::csv_loader::infer_symbol)"))
}
