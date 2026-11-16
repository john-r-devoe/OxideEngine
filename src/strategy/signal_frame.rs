//! The complete per-ticker signal set returned by one `generate_signals` call.

use std::collections::HashMap;

use crate::data::Ticker;
use crate::error::{OxideError, OxideResult};
use crate::strategy::Signal;

/// symbol -> one entry per bar of that symbol. `None` means "hold".
pub type SymbolSignals = HashMap<String, Vec<Option<Signal>>>;

/// Signals aligned to each ticker's own bars: `signal(sym, i)` applies to the
/// close of bar `i` of `sym` and fills at the open of bar `i + 1`.
/// A symbol absent from the frame holds on every bar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SignalFrame {
    signals: SymbolSignals,
}

impl SignalFrame {
    /// Rejects unknown symbols and per-symbol lists whose length differs from
    /// that ticker's bar count.
    pub fn new(signals: SymbolSignals, tickers: &[Ticker]) -> OxideResult<Self> {
        let bar_counts: HashMap<&str, usize> =
            tickers.iter().map(|t| (t.symbol(), t.len())).collect();
        for (symbol, entries) in &signals {
            let Some(&bars) = bar_counts.get(symbol.as_str()) else {
                return Err(OxideError::InvalidInput(format!(
                    "signals reference unknown symbol '{symbol}'"
                )));
            };
            if entries.len() != bars {
                return Err(OxideError::InvalidInput(format!(
                    "signals for '{symbol}' have {} entries but the ticker has {bars} bars;                      return exactly one entry (Signal or None) per bar",
                    entries.len()
                )));
            }
        }
        Ok(Self { signals })
    }

    /// Signal for bar `index` of `symbol`, or `None` to hold.
    pub fn signal(&self, symbol: &str, index: usize) -> Option<Signal> {
        self.signals
            .get(symbol)
            .and_then(|entries| entries.get(index).copied().flatten())
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
        let signals =
            SymbolSignals::from([("AAPL".to_string(), vec![None, Some(Signal::Flat), None])]);
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

    #[test]
    fn length_error_message_has_no_stray_whitespace() {
        let signals = SymbolSignals::from([("AAPL".to_string(), vec![None])]);
        let err = SignalFrame::new(signals, &[ticker("AAPL", 2)])
            .unwrap_err()
            .to_string();
        assert!(!err.contains("  "), "{err:?}");
    }
}
