//! The pure-Rust event loop. Never calls into Python.
//!
//! Loop to implement, once per timeline step `t`:
//! 1. fill orders queued at `t-1` at this step's bar open (`FillSimulator`),
//!    applying each fill to the `Portfolio` and collecting closed trades;
//! 2. mark the portfolio at each symbol's latest close -> equity point;
//! 3. for every signal in row `t`, `sizing::resolve_order` against current
//!    equity and queue the order for `t+1`.
//!
//! At the end: close open positions at the last close, then `metrics::summarize`.

use std::collections::HashSet;

use crate::backtest::{BacktestConfig, BacktestResult};
use crate::data::Ticker;
use crate::error::{OxideError, OxideResult};
use crate::strategy::SignalFrame;

/// Rejects an empty universe or duplicated symbols. Runs before the strategy is called.
pub fn validate_universe(tickers: &[Ticker]) -> OxideResult<HashSet<String>> {
    if tickers.is_empty() {
        return Err(OxideError::InvalidInput(
            "data must contain at least one Ticker".into(),
        ));
    }
    let mut symbols = HashSet::with_capacity(tickers.len());
    for ticker in tickers {
        if !symbols.insert(ticker.symbol().to_string()) {
            return Err(OxideError::InvalidInput(format!(
                "duplicate symbol '{}' in data",
                ticker.symbol()
            )));
        }
    }
    Ok(symbols)
}

/// Master timeline: sorted union of every ticker's timestamps. Signal row `i`
/// applies at `timeline[i]`, so `signals.len()` must equal `timeline.len()`.
pub fn build_timeline(_tickers: &[Ticker]) -> OxideResult<Vec<i64>> {
    Err(OxideError::NotImplemented(
        "timeline alignment (backtest::engine::build_timeline)",
    ))
}

/// Runs the full simulation over pre-computed signals.
pub fn run(
    tickers: &[Ticker],
    _signals: &SignalFrame,
    _config: &BacktestConfig,
) -> OxideResult<BacktestResult> {
    let _timeline = build_timeline(tickers)?;
    Err(OxideError::NotImplemented(
        "backtest event loop (backtest::engine::run)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Bar;

    fn ticker(symbol: &str) -> Ticker {
        Ticker::new(symbol, vec![Bar::new(0, 1.0, 1.0, 1.0, 1.0, 0.0).unwrap()]).unwrap()
    }

    #[test]
    fn universe_collects_unique_symbols() {
        let symbols = validate_universe(&[ticker("A"), ticker("B")]).unwrap();
        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn universe_rejects_empty_input() {
        assert!(validate_universe(&[])
            .unwrap_err()
            .to_string()
            .contains("data"));
    }

    #[test]
    fn universe_rejects_duplicate_symbols() {
        let err = validate_universe(&[ticker("A"), ticker("A")]).unwrap_err();
        assert!(err.to_string().contains("'A'"));
    }
}
