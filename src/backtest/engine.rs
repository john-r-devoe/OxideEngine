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

/// Master timeline: sorted, deduplicated union of every ticker's timestamps.
/// Internal to the engine: it orders events across tickers and defines the
/// equity-curve steps. Strategies never see it (signals are per-ticker).
pub fn build_timeline(tickers: &[Ticker]) -> Vec<i64> {
    let mut timeline: Vec<i64> = tickers
        .iter()
        .flat_map(|t| t.bars().iter().map(|bar| bar.timestamp))
        .collect();
    timeline.sort_unstable();
    timeline.dedup();
    timeline
}

/// Runs the full simulation over pre-computed signals.
pub fn run(
    tickers: &[Ticker],
    _signals: &SignalFrame,
    _config: &BacktestConfig,
) -> OxideResult<BacktestResult> {
    let _timeline = build_timeline(tickers);
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

    fn ticker_at(symbol: &str, timestamps: &[i64]) -> Ticker {
        let bars = timestamps
            .iter()
            .map(|&t| Bar::new(t, 1.0, 1.0, 1.0, 1.0, 0.0).unwrap())
            .collect();
        Ticker::new(symbol, bars).unwrap()
    }

    #[test]
    fn timeline_of_one_ticker_is_its_timestamps() {
        let timeline = build_timeline(&[ticker_at("A", &[1, 2, 3])]);
        assert_eq!(timeline, vec![1, 2, 3]);
    }

    #[test]
    fn timeline_is_sorted_deduplicated_union() {
        let tickers = [ticker_at("A", &[1, 3, 5]), ticker_at("B", &[2, 3, 6])];
        assert_eq!(build_timeline(&tickers), vec![1, 2, 3, 5, 6]);
    }

    #[test]
    fn timeline_of_subset_equals_superset() {
        let tickers = [ticker_at("A", &[1, 2, 3, 4]), ticker_at("B", &[2, 4])];
        assert_eq!(build_timeline(&tickers), vec![1, 2, 3, 4]);
    }

    #[test]
    fn universe_rejects_duplicate_symbols() {
        let err = validate_universe(&[ticker("A"), ticker("A")]).unwrap_err();
        assert!(err.to_string().contains("'A'"));
    }

    // ---- engine::run ----------------------------------------------------

    use crate::strategy::{Signal, SymbolSignals};

    const EPS: f64 = 1e-9;

    /// Bar with the given open/close and a high/low envelope around them.
    fn ohlc(timestamp: i64, open: f64, close: f64) -> Bar {
        let (lo, hi) = (open.min(close), open.max(close));
        Bar::new(timestamp, open, hi + 1.0, lo * 0.5, close, 0.0).unwrap()
    }

    fn series(symbol: &str, bars: &[(i64, f64, f64)]) -> Ticker {
        let bars = bars.iter().map(|&(t, o, c)| ohlc(t, o, c)).collect();
        Ticker::new(symbol, bars).unwrap()
    }

    fn costless(cash: f64) -> BacktestConfig {
        BacktestConfig::new(cash, 0.0, 0.0, 0.0, 252.0).unwrap()
    }

    fn frame(tickers: &[Ticker], entries: &[(&str, usize, Signal)]) -> SignalFrame {
        let mut signals: SymbolSignals = tickers
            .iter()
            .map(|t| (t.symbol().to_string(), vec![None; t.len()]))
            .collect();
        for &(symbol, index, signal) in entries {
            signals.get_mut(symbol).unwrap()[index] = Some(signal);
        }
        SignalFrame::new(signals, tickers).unwrap()
    }

    fn equities(result: &BacktestResult) -> Vec<f64> {
        result.equity_curve.iter().map(|p| p.equity).collect()
    }

    fn five_bars() -> Ticker {
        series(
            "A",
            &[(0, 10.0, 10.0), (1, 10.0, 11.0), (2, 11.0, 12.0), (3, 12.0, 12.0), (4, 12.0, 13.0)],
        )
    }

    #[test]
    fn round_trip_fills_at_next_open_and_marks_at_close() {
        let tickers = [five_bars()];
        let signals = frame(&tickers, &[("A", 0, Signal::Long(1.0)), ("A", 2, Signal::Flat)]);

        let result = run(&tickers, &signals, &costless(1_000.0)).unwrap();

        assert_eq!(equities(&result), vec![1_000.0, 1_100.0, 1_200.0, 1_200.0, 1_200.0]);
        assert_eq!(result.trades.len(), 1);
        let trade = &result.trades[0];
        assert_eq!((trade.entry_timestamp, trade.exit_timestamp), (1, 3));
        assert_eq!((trade.entry_price, trade.exit_price), (10.0, 12.0));
        assert!((trade.quantity - 100.0).abs() < EPS);
        assert!((trade.pnl - 200.0).abs() < EPS);
        assert_eq!(trade.bars_held, 2);
        assert!((result.final_equity - 1_200.0).abs() < EPS);
        assert!((result.metrics.total_return - 0.2).abs() < EPS);
        assert!((result.metrics.exposure - 0.4).abs() < EPS);
    }

    #[test]
    fn equity_curve_has_one_point_per_timeline_step_starting_at_cash() {
        let tickers = [series("A", &[(0, 1.0, 1.0), (2, 1.0, 1.0)]), series("B", &[(1, 1.0, 1.0)])];
        let result = run(&tickers, &frame(&tickers, &[]), &costless(500.0)).unwrap();
        let timestamps: Vec<i64> = result.equity_curve.iter().map(|p| p.timestamp).collect();
        assert_eq!(timestamps, vec![0, 1, 2]);
        assert_eq!(result.equity_curve[0].equity, 500.0);
        let dd_timestamps: Vec<i64> = result.drawdown_curve.iter().map(|p| p.timestamp).collect();
        assert_eq!(dd_timestamps, timestamps);
        assert!(result.trades.is_empty());
    }

    #[test]
    fn order_fills_at_the_symbols_next_bar_across_a_gap() {
        let a = series("A", &[(0, 1.0, 1.0), (1, 1.0, 1.0), (2, 1.0, 1.0), (3, 1.0, 1.0)]);
        let b = series("B", &[(0, 10.0, 10.0), (2, 20.0, 20.0), (3, 20.0, 30.0)]);
        let tickers = [a, b];
        let signals = frame(&tickers, &[("B", 0, Signal::Long(0.5))]);

        let result = run(&tickers, &signals, &costless(1_000.0)).unwrap();

        // B has no bar at t=1: nothing fills there; the order fills at B's t=2 open (20).
        assert_eq!(equities(&result)[1], 1_000.0);
        let trade = &result.trades[0];
        assert_eq!(trade.entry_timestamp, 2);
        assert_eq!(trade.entry_price, 20.0);
        assert!((trade.quantity - 25.0).abs() < EPS); // 0.5 * 1000 / close(t=0)=10
    }

    #[test]
    fn signal_on_a_symbols_last_bar_never_fills() {
        let tickers = [five_bars()];
        let signals = frame(&tickers, &[("A", 4, Signal::Long(1.0))]);
        let result = run(&tickers, &signals, &costless(1_000.0)).unwrap();
        assert!(result.trades.is_empty());
        assert_eq!(result.final_equity, 1_000.0);
    }

    #[test]
    fn simultaneous_signals_are_sized_against_one_equity_snapshot() {
        let a = series("A", &[(0, 10.0, 10.0), (1, 10.0, 10.0)]);
        let b = series("B", &[(0, 20.0, 20.0), (1, 20.0, 20.0)]);
        let tickers = [a, b];
        let signals = frame(&tickers, &[("A", 0, Signal::Long(0.5)), ("B", 0, Signal::Long(0.5))]);

        let result = run(&tickers, &signals, &costless(1_000.0)).unwrap();

        let quantities: Vec<(String, f64)> = result
            .trades
            .iter()
            .map(|t| (t.symbol.clone(), t.quantity))
            .collect();
        assert_eq!(quantities, vec![("A".into(), 50.0), ("B".into(), 25.0)]);
    }

    #[test]
    fn open_positions_are_liquidated_at_last_close_with_costs() {
        let tickers = [five_bars()];
        let config = BacktestConfig::new(1_000.0, 0.001, 0.01, 0.0, 252.0).unwrap();
        let signals = frame(&tickers, &[("A", 0, Signal::Long(0.5))]);

        let result = run(&tickers, &signals, &config).unwrap();

        let trade = &result.trades[0];
        assert!((trade.entry_price - 10.0 * 1.01).abs() < EPS);
        assert!((trade.exit_price - 13.0 * 0.99).abs() < EPS);
        assert_eq!(trade.exit_timestamp, 4);
        let last = result.equity_curve.last().unwrap().equity;
        assert!((result.final_equity - last).abs() < EPS);
        let quantity = 50.0;
        let expected = 1_000.0 - quantity * 10.1 * 1.001 + quantity * 12.87 * 0.999;
        assert!((result.final_equity - expected).abs() < 1e-6, "{}", result.final_equity);
    }

    #[test]
    fn short_profits_when_price_falls() {
        let tickers = [series("A", &[(0, 10.0, 10.0), (1, 10.0, 8.0), (2, 8.0, 8.0)])];
        let signals = frame(&tickers, &[("A", 0, Signal::Short(1.0)), ("A", 1, Signal::Flat)]);
        let result = run(&tickers, &signals, &costless(1_000.0)).unwrap();
        assert!((result.final_equity - 1_200.0).abs() < EPS);
        assert_eq!(result.trades[0].side, crate::portfolio::TradeSide::Short);
    }
}
