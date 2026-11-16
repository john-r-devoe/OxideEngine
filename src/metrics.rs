//! Performance statistics computed from the equity curve and trade list.
//!
//! Conventions: `returns` are simple per-period returns of the equity curve;
//! ratios are annualized with `periods_per_year`; drawdowns are positive fractions.
//! Every function is total: short or degenerate inputs yield `0.0` rather than
//! NaN or a panic (see each doc comment for the exact rule).

use crate::backtest::result::{EquityPoint, PerformanceMetrics};
use crate::error::OxideResult;
use crate::portfolio::Trade;

/// Simple period-over-period returns of an equity series (`len - 1` values).
/// A step from non-positive equity has return `0.0`.
pub fn period_returns(equity: &[f64]) -> Vec<f64> {
    equity
        .windows(2)
        .map(|w| if w[0] > 0.0 { w[1] / w[0] - 1.0 } else { 0.0 })
        .collect()
}

/// `last / first - 1`, or `0.0` for fewer than two points or non-positive start.
pub fn total_return(equity: &[f64]) -> f64 {
    match (equity.first(), equity.last()) {
        (Some(&first), Some(&last)) if equity.len() >= 2 && first > 0.0 => last / first - 1.0,
        _ => 0.0,
    }
}

/// CAGR over `len - 1` periods; `-1.0` if equity ends at or below zero.
pub fn annualized_return(equity: &[f64], periods_per_year: f64) -> f64 {
    let (Some(&first), Some(&last)) = (equity.first(), equity.last()) else {
        return 0.0;
    };
    let periods = equity.len().saturating_sub(1);
    if periods == 0 || first <= 0.0 {
        return 0.0;
    }
    if last <= 0.0 {
        return -1.0;
    }
    (last / first).powf(periods_per_year / periods as f64) - 1.0
}

/// Annualized sample standard deviation of `returns` (`0.0` below two returns).
pub fn annualized_volatility(returns: &[f64], periods_per_year: f64) -> f64 {
    sample_std(returns) * periods_per_year.sqrt()
}

/// Annualized mean excess return over its sample std; `0.0` when the std is zero.
pub fn sharpe_ratio(returns: &[f64], risk_free_rate: f64, periods_per_year: f64) -> f64 {
    let excess = excess_returns(returns, risk_free_rate, periods_per_year);
    ratio(mean(&excess), sample_std(&excess), periods_per_year)
}

/// Annualized mean excess return over downside deviation
/// (`sqrt(mean(min(excess, 0)^2))`); `0.0` when there is no downside.
pub fn sortino_ratio(returns: &[f64], risk_free_rate: f64, periods_per_year: f64) -> f64 {
    let excess = excess_returns(returns, risk_free_rate, periods_per_year);
    let downside: Vec<f64> = excess.iter().map(|r| r.min(0.0).powi(2)).collect();
    ratio(mean(&excess), mean(&downside).sqrt(), periods_per_year)
}

/// Drawdown fraction at every point: `1 - equity / running_peak` (`0.0` while
/// the running peak is non-positive).
pub fn drawdown_series(equity: &[f64]) -> Vec<f64> {
    let mut peak = f64::NEG_INFINITY;
    equity
        .iter()
        .map(|&value| {
            peak = peak.max(value);
            if peak > 0.0 {
                1.0 - value / peak
            } else {
                0.0
            }
        })
        .collect()
}

/// Largest drawdown (fraction) and the longest peak-to-recovery span in bars.
/// A drawdown that never recovers lasts until the final bar.
pub fn max_drawdown(equity: &[f64]) -> (f64, usize) {
    let deepest = drawdown_series(equity).into_iter().fold(0.0, f64::max);
    (deepest, longest_drawdown_span(equity))
}

fn longest_drawdown_span(equity: &[f64]) -> usize {
    let Some(last_index) = equity.len().checked_sub(1) else {
        return 0;
    };
    let mut peak = f64::NEG_INFINITY;
    let mut peak_index = 0;
    let mut longest = 0;
    for (index, &value) in equity.iter().enumerate() {
        if value >= peak {
            longest = longest.max(index - peak_index);
            peak = value;
            peak_index = index;
        }
    }
    longest.max(last_index - peak_index)
}

/// `annualized_return / max_drawdown`; `0.0` without a drawdown.
pub fn calmar_ratio(annualized_return: f64, max_drawdown: f64) -> f64 {
    if max_drawdown > 0.0 {
        annualized_return / max_drawdown
    } else {
        0.0
    }
}

/// Fraction of trades with `pnl > 0` (`0.0` without trades).
pub fn win_rate(trades: &[Trade]) -> f64 {
    if trades.is_empty() {
        return 0.0;
    }
    trades.iter().filter(|t| t.pnl > 0.0).count() as f64 / trades.len() as f64
}

/// Gross profit / gross loss. `0.0` without profit; `+inf` with profit and no loss.
pub fn profit_factor(trades: &[Trade]) -> f64 {
    let profit: f64 = trades.iter().map(|t| t.pnl.max(0.0)).sum();
    let loss: f64 = trades.iter().map(|t| (-t.pnl).max(0.0)).sum();
    match (profit > 0.0, loss > 0.0) {
        (false, _) => 0.0,
        (true, false) => f64::INFINITY,
        (true, true) => profit / loss,
    }
}

/// Computes every statistic in [`PerformanceMetrics`]. `steps_exposed` is the
/// number of equity-curve steps with any open position.
pub fn summarize(
    equity_curve: &[EquityPoint],
    trades: &[Trade],
    risk_free_rate: f64,
    periods_per_year: f64,
    steps_exposed: usize,
) -> OxideResult<PerformanceMetrics> {
    let equity: Vec<f64> = equity_curve.iter().map(|p| p.equity).collect();
    let returns = period_returns(&equity);
    let annualized = annualized_return(&equity, periods_per_year);
    let (max_dd, max_dd_duration) = max_drawdown(&equity);
    let pnls: Vec<f64> = trades.iter().map(|t| t.pnl).collect();
    Ok(PerformanceMetrics {
        total_return: total_return(&equity),
        annualized_return: annualized,
        annualized_volatility: annualized_volatility(&returns, periods_per_year),
        sharpe: sharpe_ratio(&returns, risk_free_rate, periods_per_year),
        sortino: sortino_ratio(&returns, risk_free_rate, periods_per_year),
        calmar: calmar_ratio(annualized, max_dd),
        max_drawdown: max_dd,
        max_drawdown_duration: max_dd_duration,
        win_rate: win_rate(trades),
        profit_factor: profit_factor(trades),
        avg_trade_pnl: mean(&pnls),
        exposure: fraction(steps_exposed, equity.len()),
    })
}

fn excess_returns(returns: &[f64], risk_free_rate: f64, periods_per_year: f64) -> Vec<f64> {
    let per_period = risk_free_rate / periods_per_year;
    returns.iter().map(|r| r - per_period).collect()
}

/// `mean / dispersion * sqrt(periods_per_year)`, or `0.0` when dispersion is ~0.
fn ratio(mean: f64, dispersion: f64, periods_per_year: f64) -> f64 {
    if dispersion > f64::EPSILON {
        mean / dispersion * periods_per_year.sqrt()
    } else {
        0.0
    }
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn sample_std(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let m = mean(values);
    let variance = values.iter().map(|v| (v - m).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

fn fraction(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
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
        assert!(approx(
            annualized_return(&[100.0, 121.0], 0.5),
            1.21f64.powf(0.5) - 1.0
        ));
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
        assert!(approx(
            annualized_volatility(&[0.1, -0.1], 4.0),
            0.282_842_7
        ));
    }

    #[test]
    fn sharpe_uses_per_period_risk_free_rate() {
        assert!(approx(sharpe_ratio(&[0.1, 0.3], 0.0, 4.0), 2.828_427));
        assert!(approx(sharpe_ratio(&[0.1, 0.3], 0.4, 4.0), 1.414_214));
        assert_eq!(sharpe_ratio(&[0.1, 0.1], 0.0, 4.0), 0.0);
    }

    #[test]
    fn sortino_uses_downside_deviation() {
        assert!(approx(
            sortino_ratio(&[0.1, -0.1, 0.3], 0.0, 1.0),
            1.732_051
        ));
        assert_eq!(sortino_ratio(&[0.1, 0.2], 0.0, 1.0), 0.0);
    }

    #[test]
    fn drawdowns_are_positive_fractions_from_running_peak() {
        let dd = drawdown_series(&CURVE);
        assert!(
            approx(dd[0], 0.0) && approx(dd[1], 0.0) && approx(dd[2], 0.1) && approx(dd[3], 0.0)
        );
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
            .map(|(i, &equity)| EquityPoint {
                timestamp: i as i64,
                equity,
            })
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
