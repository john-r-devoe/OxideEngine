//! The complete signal matrix returned by one `generate_signals` call.

use std::collections::{HashMap, HashSet};

use crate::error::{OxideError, OxideResult};
use crate::strategy::Signal;

/// Signals for one timestep: symbol -> signal. A missing symbol means "hold".
pub type SignalRow = HashMap<String, Signal>;

/// All signals for all timesteps. Row `i` applies to timeline step `i`
/// (see [`crate::backtest::engine::build_timeline`]).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignalFrame {
    rows: Vec<SignalRow>,
}

impl SignalFrame {
    /// Builds a frame, rejecting any symbol that is not part of the backtest universe.
    pub fn new(rows: Vec<SignalRow>, universe: &HashSet<String>) -> OxideResult<Self> {
        for (index, row) in rows.iter().enumerate() {
            if let Some(unknown) = row.keys().find(|symbol| !universe.contains(*symbol)) {
                return Err(OxideError::InvalidInput(format!(
                    "signals at index {index} reference unknown symbol '{unknown}'"
                )));
            }
        }
        Ok(Self { rows })
    }

    pub fn rows(&self) -> &[SignalRow] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn universe(symbols: &[&str]) -> HashSet<String> {
        symbols.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn accepts_rows_with_known_symbols_and_holds() {
        let rows = vec![
            SignalRow::new(),
            SignalRow::from([("AAPL".to_string(), Signal::Flat)]),
        ];
        let frame = SignalFrame::new(rows, &universe(&["AAPL"])).unwrap();
        assert_eq!(frame.len(), 2);
    }

    #[test]
    fn rejects_unknown_symbol_with_row_index() {
        let rows = vec![
            SignalRow::new(),
            SignalRow::from([("TSLA".to_string(), Signal::Flat)]),
        ];
        let err = SignalFrame::new(rows, &universe(&["AAPL"]))
            .unwrap_err()
            .to_string();
        assert!(err.contains("index 1") && err.contains("TSLA"), "{err}");
    }
}
