//! Performance statistics computed from the equity curve and trade list.
//!
//! Conventions: `returns` are simple per-period returns of the equity curve;
//! ratios are annualized with `periods_per_year`; drawdowns are positive fractions.

use crate::backtest::result::{EquityPoint, PerformanceMetrics};
use crate::error::{OxideError, OxideResult};
use crate::portfolio::Trade;

/// Simple period-over-period returns of an equity series.
pub fn period_returns(_equity: &[f64]) -> Vec<f64> {
    todo!("period returns (metrics::period_returns)")
}

pub fn total_return(_equity: &[f64]) -> f64 {
    todo!("total return (metrics::total_return)")
}

pub fn annualized_return(_equity: &[f64], _periods_per_year: f64) -> f64 {
    todo!("CAGR (metrics::annualized_return)")
}

pub fn annualized_volatility(_returns: &[f64], _periods_per_year: f64) -> f64 {
    todo!("volatility (metrics::annualized_volatility)")
}

pub fn sharpe_ratio(_returns: &[f64], _risk_free_rate: f64, _periods_per_year: f64) -> f64 {
    todo!("Sharpe ratio (metrics::sharpe_ratio)")
}

pub fn sortino_ratio(_returns: &[f64], _risk_free_rate: f64, _periods_per_year: f64) -> f64 {
    todo!("Sortino ratio (metrics::sortino_ratio)")
}

/// Drawdown fraction at every point: `1 - equity / running_peak`.
pub fn drawdown_series(_equity: &[f64]) -> Vec<f64> {
    todo!("drawdown series (metrics::drawdown_series)")
}

/// Largest drawdown (fraction) and its duration in bars (peak to recovery).
pub fn max_drawdown(_equity: &[f64]) -> (f64, usize) {
    todo!("max drawdown (metrics::max_drawdown)")
}

pub fn calmar_ratio(_annualized_return: f64, _max_drawdown: f64) -> f64 {
    todo!("Calmar ratio (metrics::calmar_ratio)")
}

/// Fraction of trades with `pnl > 0`.
pub fn win_rate(_trades: &[Trade]) -> f64 {
    todo!("win rate (metrics::win_rate)")
}

/// Gross profit / gross loss.
pub fn profit_factor(_trades: &[Trade]) -> f64 {
    todo!("profit factor (metrics::profit_factor)")
}

/// Computes every statistic in [`PerformanceMetrics`].
pub fn summarize(
    _equity_curve: &[EquityPoint],
    _trades: &[Trade],
    _risk_free_rate: f64,
    _periods_per_year: f64,
) -> OxideResult<PerformanceMetrics> {
    Err(OxideError::NotImplemented(
        "performance metrics (metrics::summarize)",
    ))
}
