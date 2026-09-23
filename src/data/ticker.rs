//! A symbol and its chronologically ordered bars.

use std::sync::Arc;

use crate::data::Bar;
use crate::error::{OxideError, OxideResult};

/// Immutable bar series for one symbol. Cloning is O(1) (shared `Arc`).
#[derive(Debug, Clone, PartialEq)]
pub struct Ticker {
    symbol: String,
    bars: Arc<[Bar]>,
}

impl Ticker {
    /// Rejects blank symbols and empty series.
    ///
    /// TODO(data): also require strictly increasing timestamps once the CSV
    /// loader defines its sort/dedup policy.
    pub fn new(symbol: impl Into<String>, bars: Vec<Bar>) -> OxideResult<Self> {
        let symbol = symbol.into().trim().to_string();
        if symbol.is_empty() {
            return Err(OxideError::InvalidTicker("symbol must not be blank".into()));
        }
        if bars.is_empty() {
            return Err(OxideError::InvalidTicker(format!(
                "bars for '{symbol}' must not be empty"
            )));
        }
        Ok(Self {
            symbol,
            bars: bars.into(),
        })
    }

    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    pub fn bars(&self) -> &[Bar] {
        &self.bars
    }

    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Always false for a constructed ticker; provided for clippy's `len_without_is_empty`.
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one_bar() -> Vec<Bar> {
        vec![Bar::new(0, 1.0, 1.0, 1.0, 1.0, 0.0).unwrap()]
    }

    #[test]
    fn trims_symbol_whitespace() {
        let ticker = Ticker::new("  AAPL ", one_bar()).unwrap();
        assert_eq!(ticker.symbol(), "AAPL");
        assert_eq!(ticker.len(), 1);
    }

    #[test]
    fn rejects_blank_symbol() {
        assert!(Ticker::new("   ", one_bar()).is_err());
    }

    #[test]
    fn rejects_empty_bars() {
        let err = Ticker::new("AAPL", vec![]).unwrap_err();
        assert!(err.to_string().contains("bars"));
    }
}
