use chrono::NaiveDateTime;
use pyo3::prelude::*;

#[pyclass]
pub struct Bar {
    pub datetime: NaiveDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub vol: f64,
}