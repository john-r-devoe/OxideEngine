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
    use crate::data::{Bar, Ticker};

    fn ticker(symbol: &str, n_bars: i64) -> Ticker {
        let bars = (0..n_bars)
            .map(|t| Bar::new(t, 1.0, 1.0, 1.0, 1.0, 0.0).unwrap())
            .collect();
        Ticker::new(symbol, bars).unwrap()
    }

    #[test]
    fn looks_up_signal_by_symbol_and_bar_index() {
        let signals = SymbolSignals::from([(
            "AAPL".to_string(),
            vec![None, Some(Signal::Flat), None],
        )]);
        let frame = SignalFrame::new(signals, &[ticker("AAPL", 3)]).unwrap();
        assert_eq!(frame.signal("AAPL", 1), Some(Signal::Flat));
        assert_eq!(frame.signal("AAPL", 0), None);
    }

    #[test]
    fn omitted_symbol_holds_on_every_bar() {
        let signals = SymbolSignals::from([("AAPL".to_string(), vec![None])]);
        let frame = SignalFrame::new(signals, &[ticker("AAPL", 1), ticker("MSFT", 4)]).unwrap();
        assert_eq!(frame.signal("MSFT", 3), None);
    }

    #[test]
    fn rejects_unknown_symbol() {
        let signals = SymbolSignals::from([("TSLA".to_string(), vec![None])]);
        let err = SignalFrame::new(signals, &[ticker("AAPL", 1)])
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown symbol 'TSLA'"), "{err}");
    }

    #[test]
    fn rejects_length_that_differs_from_bar_count() {
        let signals = SymbolSignals::from([("AAPL".to_string(), vec![None, None])]);
        let err = SignalFrame::new(signals, &[ticker("AAPL", 3)])
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("'AAPL'") && err.contains("2 entries") && err.contains("3 bars"),
            "{err}"
        );
    }
}
