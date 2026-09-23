//! Python wrapper for `Signal`. Construct via the static factories only.

use pyo3::prelude::*;

use crate::strategy::Signal;

/// Target exposure as a fraction of equity in [0, 1]. Use `None` to hold.
#[pyclass(name = "Signal", module = "oxide_engine", frozen, eq, skip_from_py_object)]
#[derive(Debug, Clone, PartialEq)]
pub struct PySignal {
    pub(crate) inner: Signal,
}

#[pymethods]
impl PySignal {
    #[staticmethod]
    fn long(weight: f64) -> PyResult<Self> {
        Ok(Self { inner: Signal::long(weight)? })
    }

    #[staticmethod]
    fn short(weight: f64) -> PyResult<Self> {
        Ok(Self { inner: Signal::short(weight)? })
    }

    #[staticmethod]
    fn flat() -> Self {
        Self { inner: Signal::flat() }
    }

    #[staticmethod]
    fn scale_in(weight: f64) -> PyResult<Self> {
        Ok(Self { inner: Signal::scale_in(weight)? })
    }

    #[staticmethod]
    fn scale_out(weight: f64) -> PyResult<Self> {
        Ok(Self { inner: Signal::scale_out(weight)? })
    }

    /// One of "long", "short", "flat", "scale_in", "scale_out".
    #[getter]
    fn kind(&self) -> &'static str {
        self.inner.kind()
    }

    /// Fraction of equity, or None for `flat`.
    #[getter]
    fn weight(&self) -> Option<f64> {
        self.inner.weight()
    }

    fn __repr__(&self) -> String {
        match self.inner.weight() {
            Some(w) => format!("Signal.{}({w:?})", self.inner.kind()),
            None => format!("Signal.{}()", self.inner.kind()),
        }
    }
}
