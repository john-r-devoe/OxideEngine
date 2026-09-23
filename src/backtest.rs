//! Backtest configuration, the event loop, and its results.

pub mod config;
pub mod engine;
pub mod result;

pub use config::BacktestConfig;
pub use result::{BacktestResult, EquityPoint, PerformanceMetrics};
