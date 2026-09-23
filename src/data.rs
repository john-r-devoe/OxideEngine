//! Market data: canonical bars, tickers, and source-agnostic CSV ingestion.

pub mod bar;
pub mod csv_loader;
pub mod schema;
pub mod ticker;
pub mod timestamp;

pub use bar::Bar;
pub use schema::{CanonicalField, ColumnSchema};
pub use ticker::Ticker;
