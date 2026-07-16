use std::collections::VecDeque;

use pyo3::prelude::*;

use crate::data::bar::Bar;

#[pyclass]
pub struct Ticker {
    pub symbol: String,
    pub bars: VecDeque<Bar>,
}