//! PyO3 layer. The ONLY place allowed to use `#[pyclass]` / `#[pyfunction]`.
//!
//! Each `Py*` type is a thin wrapper around a pure-Rust domain type; all
//! validation lives in the domain constructors and surfaces via `OxideError -> PyErr`.

pub mod backtest;
pub mod data;
pub mod signal;
