// lib.rs — owns all Python structure
use pyo3::prelude::*;
pub mod data;

#[pymodule]
mod oxide_core {
    use pyo3::prelude::*;

    #[pymodule]
    mod data_loader {
        #[pymodule_export]
        use crate::data::data_loader::from_csv;
    }
}