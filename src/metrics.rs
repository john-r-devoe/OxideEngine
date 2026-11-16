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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portfolio::TradeSide;

    const EPS: f64 = 1e-6;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    fn trade(pnl: f64) -> Trade {
        Trade {
            symbol: "A".into(),
            side: TradeSide::Long,
            quantity: 1.0,
            entry_timestamp: 0,
            exit_timestamp: 1,
            entry_price: 1.0,
            exit_price: 1.0,
            pnl,
            return_pct: pnl,
            commission: 0.0,
            bars_held: 1,
        }
    }

    const CURVE: [f64; 4] = [100.0, 110.0, 99.0, 120.0];

    #[test]
    fn period_returns_are_simple_returns() {
        let r = period_returns(&CURVE);
        assert_eq!(r.len(), 3);
        assert!(approx(r[0], 0.1) && approx(r[1], -0.1) && approx(r[2], 120.0 / 99.0 - 1.0));
        assert!(period_returns(&[100.0]).is_empty());
    }

    #[test]
    fn total_and_annualized_return() {
        assert!(approx(total_return(&CURVE), 0.2));
        assert!(approx(annualized_return(&CURVE, 3.0), 0.2));
        assert!(approx(annualized_return(&[100.0, 121.0], 0.5), 1.21f64.powf(0.5) - 1.0));
    }

    #[test]
    fn returns_are_zero_for_short_series() {
        assert_eq!(total_return(&[]), 0.0);
        assert_eq!(annualized_return(&[100.0], 252.0), 0.0);
        assert_eq!(annualized_volatility(&[0.1], 252.0), 0.0);
    }

    #[test]
    fn annualized_return_is_minus_one_when_wiped_out() {
        assert_eq!(annualized_return(&[100.0, -5.0], 252.0), -1.0);
    }

    #[test]
    fn volatility_uses_sample_std() {
        assert!(approx(annualized_volatility(&[0.1, -0.1], 4.0), 0.282_842_7));
    }

    #[test]
    fn sharpe_uses_per_period_risk_free_rate() {
        assert!(approx(sharpe_ratio(&[0.1, 0.3], 0.0, 4.0), 2.828_427));
        assert!(approx(sharpe_ratio(&[0.1, 0.3], 0.4, 4.0), 1.414_214));
        assert_eq!(sharpe_ratio(&[0.1, 0.1], 0.0, 4.0), 0.0);
    }

    #[test]
    fn sortino_uses_downside_deviation() {
        assert!(approx(sortino_ratio(&[0.1, -0.1, 0.3], 0.0, 1.0), 1.732_051));
        assert_eq!(sortino_ratio(&[0.1, 0.2], 0.0, 1.0), 0.0);
    }

    #[test]
    fn drawdowns_are_positive_fractions_from_running_peak() {
        let dd = drawdown_series(&CURVE);
        assert!(approx(dd[0], 0.0) && approx(dd[1], 0.0) && approx(dd[2], 0.1) && approx(dd[3], 0.0));
        let (max, duration) = max_drawdown(&CURVE);
        assert!(approx(max, 0.1));
        assert_eq!(duration, 2);
    }

    #[test]
    fn unrecovered_drawdown_lasts_to_the_end() {
        let (max, duration) = max_drawdown(&[100.0, 90.0, 95.0]);
        assert!(approx(max, 0.1));
        assert_eq!(duration, 2);
    }

    #[test]
    fn max_drawdown_duration_is_longest_span() {
        // short deep dip (1 bar) vs long shallow dip (3 bars)
        let (max, duration) = max_drawdown(&[100.0, 50.0, 100.0, 99.0, 98.0, 99.0, 101.0]);
        assert!(approx(max, 0.5));
        assert_eq!(duration, 4);
    }

    #[test]
    fn calmar_is_zero_without_drawdown() {
        assert!(approx(calmar_ratio(0.2, 0.1), 2.0));
        assert_eq!(calmar_ratio(0.2, 0.0), 0.0);
    }

    #[test]
    fn trade_statistics() {
        let trades = [trade(10.0), trade(-5.0), trade(20.0), trade(0.0)];
        assert!(approx(win_rate(&trades), 0.5));
        assert!(approx(profit_factor(&trades), 6.0));
        assert_eq!(win_rate(&[]), 0.0);
        assert_eq!(profit_factor(&[]), 0.0);
        assert_eq!(profit_factor(&[trade(3.0)]), f64::INFINITY);
    }

    #[test]
    fn summarize_fills_every_metric() {
        let curve: Vec<EquityPoint> = CURVE
            .iter()
            .enumerate()
            .map(|(i, &equity)| EquityPoint { timestamp: i as i64, equity })
            .collect();
        let m = summarize(&curve, &[trade(10.0), trade(-4.0)], 0.0, 3.0, 2).unwrap();
        assert!(approx(m.total_return, 0.2));
        assert!(approx(m.annualized_return, 0.2));
        assert!(approx(m.max_drawdown, 0.1));
        assert_eq!(m.max_drawdown_duration, 2);
        assert!(approx(m.calmar, 2.0));
        assert!(approx(m.win_rate, 0.5));
        assert!(approx(m.profit_factor, 2.5));
        assert!(approx(m.avg_trade_pnl, 3.0));
        assert!(approx(m.exposure, 0.5));
        assert!(m.sharpe.is_finite() && m.sortino.is_finite() && m.annualized_volatility > 0.0);
    }

    #[test]
    fn summarize_of_empty_inputs_is_all_zero() {
        let m = summarize(&[], &[], 0.0, 252.0, 0).unwrap();
        assert_eq!(m, PerformanceMetrics::default());
    }
}
