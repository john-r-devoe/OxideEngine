//! Everything a backtest produces — shaped for plotting and analysis in Python.

use crate::backtest::BacktestConfig;
use crate::portfolio::Trade;

/// Portfolio value at one timeline step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquityPoint {
    pub timestamp: i64,
    pub equity: f64,
}

/// Summary statistics. Ratios are annualized; drawdowns are positive fractions.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PerformanceMetrics {
    pub total_return: f64,
    pub annualized_return: f64,
    pub annualized_volatility: f64,
    pub sharpe: f64,
    pub sortino: f64,
    pub calmar: f64,
    pub max_drawdown: f64,
    /// Longest peak-to-recovery span, in bars.
    pub max_drawdown_duration: usize,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_trade_pnl: f64,
    /// Fraction of timeline steps with any open position.
    pub exposure: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BacktestResult {
    pub config: BacktestConfig,
    pub metrics: PerformanceMetrics,
    pub final_equity: f64,
    /// One point per timeline step, starting at `config.starting_cash`.
    pub equity_curve: Vec<EquityPoint>,
    /// Drawdown fraction per timeline step (same timestamps as `equity_curve`).
    pub drawdown_curve: Vec<EquityPoint>,
    pub trades: Vec<Trade>,
}

impl BacktestResult {
    pub fn num_trades(&self) -> usize {
        self.trades.len()
    }
}
