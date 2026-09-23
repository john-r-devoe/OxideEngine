//! Oxide Engine — Python strategies, Rust execution.
//!
//! Layout:
//! - [`data`]       canonical bars, tickers, CSV ingestion
//! - [`strategy`]   `Signal` / `SignalFrame` produced by the Python strategy
//! - [`execution`]  sizing (weight -> units), orders, fill simulation
//! - [`portfolio`]  cash + signed positions, round-trip trades
//! - [`metrics`]    performance statistics
//! - [`backtest`]   config, event loop, results
//! - [`bindings`]   the only module that touches PyO3 attributes
//!
//! The Python extension module `oxide_engine._core` is assembled below; the
//! pure-Python package in `python/oxide_engine/` re-exports it.

pub mod backtest;
pub mod bindings;
pub mod data;
pub mod error;
pub mod execution;
pub mod metrics;
pub mod portfolio;
pub mod strategy;

use pyo3::prelude::*;

#[pymodule]
mod _core {
    use super::*;

    #[pymodule_export]
    use crate::bindings::backtest::run_backtest;
    #[pymodule_export]
    use crate::bindings::backtest::PyBacktestConfig;
    #[pymodule_export]
    use crate::bindings::backtest::PyBacktestResult;
    #[pymodule_export]
    use crate::bindings::backtest::PyTrade;
    #[pymodule_export]
    use crate::bindings::data::PyBar;
    #[pymodule_export]
    use crate::bindings::data::PyTicker;
    #[pymodule_export]
    use crate::bindings::signal::PySignal;

    /// `oxide_engine.data_loader`
    #[pymodule]
    mod data_loader {
        #[pymodule_export]
        use crate::bindings::data::from_csv;
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add("__version__", env!("CARGO_PKG_VERSION"))
    }
}
