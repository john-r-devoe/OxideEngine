//! Python wrappers for `Bar`, `Ticker`, and `data_loader.from_csv`.

use std::collections::HashMap;
use std::path::Path;

use pyo3::exceptions::PyIndexError;
use pyo3::prelude::*;

use crate::data::csv_loader::load_csv;
use crate::data::{Bar, ColumnSchema, Ticker};

/// One OHLCV bar. `timestamp` is Unix epoch milliseconds (UTC).
#[pyclass(name = "Bar", module = "oxide_engine", frozen, eq, skip_from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct PyBar {
    pub(crate) inner: Bar,
}

#[pymethods]
impl PyBar {
    #[new]
    fn new(
        timestamp: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: Bar::new(timestamp, open, high, low, close, volume)?,
        })
    }

    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn open(&self) -> f64 {
        self.inner.open
    }

    #[getter]
    fn high(&self) -> f64 {
        self.inner.high
    }

    #[getter]
    fn low(&self) -> f64 {
        self.inner.low
    }

    #[getter]
    fn close(&self) -> f64 {
        self.inner.close
    }

    #[getter]
    fn volume(&self) -> f64 {
        self.inner.volume
    }

    fn __repr__(&self) -> String {
        let b = &self.inner;
        format!(
            "Bar(timestamp={}, open={:?}, high={:?}, low={:?}, close={:?}, volume={:?})",
            b.timestamp, b.open, b.high, b.low, b.close, b.volume
        )
    }
}

/// A symbol and its bars. Supports `len()` and (negative) indexing.
#[pyclass(
    name = "Ticker",
    module = "oxide_engine",
    frozen,
    sequence,
    skip_from_py_object
)]
#[derive(Debug, Clone)]
pub struct PyTicker {
    pub(crate) inner: Ticker,
}

#[pymethods]
impl PyTicker {
    #[new]
    fn new(symbol: &str, bars: Vec<PyRef<'_, PyBar>>) -> PyResult<Self> {
        let bars = bars.iter().map(|b| b.inner).collect();
        Ok(Self {
            inner: Ticker::new(symbol, bars)?,
        })
    }

    #[getter]
    fn symbol(&self) -> &str {
        self.inner.symbol()
    }

    /// All bars as a new list (copies every bar — prefer indexing for large series).
    #[getter]
    fn bars(&self) -> Vec<PyBar> {
        self.inner
            .bars()
            .iter()
            .map(|&inner| PyBar { inner })
            .collect()
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __getitem__(&self, index: isize) -> PyResult<PyBar> {
        let len = self.inner.len() as isize;
        let resolved = if index < 0 { index + len } else { index };
        if !(0..len).contains(&resolved) {
            return Err(PyIndexError::new_err(format!(
                "bar index {index} out of range for {len} bars"
            )));
        }
        Ok(PyBar {
            inner: self.inner.bars()[resolved as usize],
        })
    }

    fn __repr__(&self) -> String {
        format!(
            "Ticker(symbol={:?}, bars={})",
            self.inner.symbol(),
            self.inner.len()
        )
    }
}

/// Loads a CSV into a `Ticker`. `schema` maps your column names to
/// `timestamp|open|high|low|close|volume`; without it headers are auto-normalized.
#[pyfunction]
#[pyo3(signature = (path, symbol = None, schema = None))]
pub fn from_csv(
    path: &str,
    symbol: Option<&str>,
    schema: Option<HashMap<String, String>>,
) -> PyResult<PyTicker> {
    let schema = schema.map(ColumnSchema::from_mapping).transpose()?;
    let ticker = load_csv(Path::new(path), symbol, schema.as_ref())?;
    Ok(PyTicker { inner: ticker })
}
