//! Crate-wide error type and its single conversion into Python exceptions.
//!
//! Domain code returns [`OxideResult`]; the bindings layer uses `?` and the
//! `From<OxideError> for PyErr` impl below picks the Python exception class.

use pyo3::exceptions::{PyIOError, PyNotImplementedError, PyValueError};
use pyo3::PyErr;
use thiserror::Error;

pub type OxideResult<T> = Result<T, OxideError>;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum OxideError {
    #[error("could not read '{path}': {message}")]
    Io { path: String, message: String },

    #[error("invalid column schema: {0}")]
    Schema(String),

    #[error("malformed CSV at row {row}: {message}")]
    MalformedRow { row: usize, message: String },

    #[error("invalid bar: {0}")]
    InvalidBar(String),

    #[error("invalid ticker: {0}")]
    InvalidTicker(String),

    #[error("invalid signal: {0}")]
    InvalidSignal(String),

    #[error("invalid backtest config: {0}")]
    InvalidConfig(String),

    #[error("invalid backtest input: {0}")]
    InvalidInput(String),

    /// Placeholder returned by every stubbed code path in the skeleton.
    #[error("not implemented yet: {0}")]
    NotImplemented(&'static str),
}

impl From<OxideError> for PyErr {
    fn from(err: OxideError) -> PyErr {
        match err {
            OxideError::Io { .. } => PyIOError::new_err(err.to_string()),
            OxideError::NotImplemented(_) => PyNotImplementedError::new_err(err.to_string()),
            // Listed explicitly so a new variant forces a conscious mapping choice.
            OxideError::Schema(_)
            | OxideError::MalformedRow { .. }
            | OxideError::InvalidBar(_)
            | OxideError::InvalidTicker(_)
            | OxideError::InvalidSignal(_)
            | OxideError::InvalidConfig(_)
            | OxideError::InvalidInput(_) => PyValueError::new_err(err.to_string()),
        }
    }
}
