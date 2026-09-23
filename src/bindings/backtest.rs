//! `run_backtest` plus Python wrappers for config, trades, and results.
//!
//! ARCHITECTURAL RULE: `run_backtest` performs exactly ONE call into Python
//! (`strategy.generate_signals(data)`). Everything after that runs in Rust with
//! the GIL released. Never add per-bar callbacks.

use std::collections::HashSet;

use pyo3::exceptions::PyTypeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::backtest::engine;
use crate::backtest::{BacktestConfig, BacktestResult};
use crate::bindings::data::{PyBar, PyTicker};
use crate::bindings::signal::PySignal;
use crate::data::Ticker;
use crate::portfolio::Trade;
use crate::strategy::{SignalFrame, SignalRow};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Simulation parameters. `commission`/`slippage` are fractions (0.001 = 10 bps).
#[pyclass(name = "BacktestConfig", module = "oxide_engine", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyBacktestConfig {
    pub(crate) inner: BacktestConfig,
}

#[pymethods]
impl PyBacktestConfig {
    #[new]
    #[pyo3(signature = (starting_cash, commission = 0.0, slippage = 0.0, risk_free_rate = 0.0, periods_per_year = 252.0))]
    fn new(
        starting_cash: f64,
        commission: f64,
        slippage: f64,
        risk_free_rate: f64,
        periods_per_year: f64,
    ) -> PyResult<Self> {
        let inner = BacktestConfig::new(starting_cash, commission, slippage, risk_free_rate, periods_per_year)?;
        Ok(Self { inner })
    }

    #[getter]
    fn starting_cash(&self) -> f64 {
        self.inner.starting_cash
    }

    #[getter]
    fn commission(&self) -> f64 {
        self.inner.commission
    }

    #[getter]
    fn slippage(&self) -> f64 {
        self.inner.slippage
    }

    #[getter]
    fn risk_free_rate(&self) -> f64 {
        self.inner.risk_free_rate
    }

    #[getter]
    fn periods_per_year(&self) -> f64 {
        self.inner.periods_per_year
    }

    fn __repr__(&self) -> String {
        let c = &self.inner;
        format!(
            "BacktestConfig(starting_cash={:?}, commission={:?}, slippage={:?}, risk_free_rate={:?}, periods_per_year={:?})",
            c.starting_cash, c.commission, c.slippage, c.risk_free_rate, c.periods_per_year
        )
    }
}

// ---------------------------------------------------------------------------
// Trade
// ---------------------------------------------------------------------------

/// One round-trip trade. `to_dict()` feeds straight into `pandas.DataFrame`.
#[pyclass(name = "Trade", module = "oxide_engine", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyTrade {
    inner: Trade,
}

#[pymethods]
impl PyTrade {
    #[getter]
    fn symbol(&self) -> &str {
        &self.inner.symbol
    }

    /// "long" or "short".
    #[getter]
    fn side(&self) -> &'static str {
        self.inner.side.as_str()
    }

    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    #[getter]
    fn entry_timestamp(&self) -> i64 {
        self.inner.entry_timestamp
    }

    #[getter]
    fn exit_timestamp(&self) -> i64 {
        self.inner.exit_timestamp
    }

    #[getter]
    fn entry_price(&self) -> f64 {
        self.inner.entry_price
    }

    #[getter]
    fn exit_price(&self) -> f64 {
        self.inner.exit_price
    }

    #[getter]
    fn pnl(&self) -> f64 {
        self.inner.pnl
    }

    #[getter]
    fn return_pct(&self) -> f64 {
        self.inner.return_pct
    }

    #[getter]
    fn commission(&self) -> f64 {
        self.inner.commission
    }

    #[getter]
    fn bars_held(&self) -> usize {
        self.inner.bars_held
    }

    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let t = &self.inner;
        let dict = PyDict::new(py);
        dict.set_item("symbol", &t.symbol)?;
        dict.set_item("side", t.side.as_str())?;
        dict.set_item("quantity", t.quantity)?;
        dict.set_item("entry_timestamp", t.entry_timestamp)?;
        dict.set_item("exit_timestamp", t.exit_timestamp)?;
        dict.set_item("entry_price", t.entry_price)?;
        dict.set_item("exit_price", t.exit_price)?;
        dict.set_item("pnl", t.pnl)?;
        dict.set_item("return_pct", t.return_pct)?;
        dict.set_item("commission", t.commission)?;
        dict.set_item("bars_held", t.bars_held)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        let t = &self.inner;
        format!("Trade(symbol={:?}, side={:?}, pnl={:?})", t.symbol, t.side.as_str(), t.pnl)
    }
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// Metrics, equity/drawdown curves, and every trade from one backtest.
#[pyclass(name = "BacktestResult", module = "oxide_engine", frozen, skip_from_py_object)]
#[derive(Debug, Clone)]
pub struct PyBacktestResult {
    inner: BacktestResult,
}

#[pymethods]
impl PyBacktestResult {
    #[getter]
    fn sharpe(&self) -> f64 {
        self.inner.metrics.sharpe
    }

    #[getter]
    fn sortino(&self) -> f64 {
        self.inner.metrics.sortino
    }

    #[getter]
    fn calmar(&self) -> f64 {
        self.inner.metrics.calmar
    }

    /// Largest peak-to-trough decline as a positive fraction (0.25 = -25%).
    #[getter]
    fn max_drawdown(&self) -> f64 {
        self.inner.metrics.max_drawdown
    }

    /// Longest drawdown, in bars.
    #[getter]
    fn max_drawdown_duration(&self) -> usize {
        self.inner.metrics.max_drawdown_duration
    }

    #[getter]
    fn total_return(&self) -> f64 {
        self.inner.metrics.total_return
    }

    #[getter]
    fn annualized_return(&self) -> f64 {
        self.inner.metrics.annualized_return
    }

    #[getter]
    fn annualized_volatility(&self) -> f64 {
        self.inner.metrics.annualized_volatility
    }

    #[getter]
    fn win_rate(&self) -> f64 {
        self.inner.metrics.win_rate
    }

    #[getter]
    fn profit_factor(&self) -> f64 {
        self.inner.metrics.profit_factor
    }

    #[getter]
    fn avg_trade_pnl(&self) -> f64 {
        self.inner.metrics.avg_trade_pnl
    }

    /// Fraction of timeline steps with an open position.
    #[getter]
    fn exposure(&self) -> f64 {
        self.inner.metrics.exposure
    }

    #[getter]
    fn num_trades(&self) -> usize {
        self.inner.num_trades()
    }

    #[getter]
    fn starting_cash(&self) -> f64 {
        self.inner.config.starting_cash
    }

    #[getter]
    fn final_equity(&self) -> f64 {
        self.inner.final_equity
    }

    /// `[(timestamp_ms, equity), ...]` — one point per timeline step.
    #[getter]
    fn equity_curve(&self) -> Vec<(i64, f64)> {
        self.inner.equity_curve.iter().map(|p| (p.timestamp, p.equity)).collect()
    }

    /// `[(timestamp_ms, drawdown_fraction), ...]` aligned with `equity_curve`.
    #[getter]
    fn drawdown_curve(&self) -> Vec<(i64, f64)> {
        self.inner.drawdown_curve.iter().map(|p| (p.timestamp, p.equity)).collect()
    }

    #[getter]
    fn trades(&self) -> Vec<PyTrade> {
        self.inner.trades.iter().cloned().map(|inner| PyTrade { inner }).collect()
    }

    #[getter]
    fn config(&self) -> PyBacktestConfig {
        PyBacktestConfig { inner: self.inner.config }
    }

    /// Scalar metrics as a dict (for printing or `pd.Series(result.summary())`).
    fn summary<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let m = &self.inner.metrics;
        let dict = PyDict::new(py);
        dict.set_item("starting_cash", self.inner.config.starting_cash)?;
        dict.set_item("final_equity", self.inner.final_equity)?;
        dict.set_item("total_return", m.total_return)?;
        dict.set_item("annualized_return", m.annualized_return)?;
        dict.set_item("annualized_volatility", m.annualized_volatility)?;
        dict.set_item("sharpe", m.sharpe)?;
        dict.set_item("sortino", m.sortino)?;
        dict.set_item("calmar", m.calmar)?;
        dict.set_item("max_drawdown", m.max_drawdown)?;
        dict.set_item("max_drawdown_duration", m.max_drawdown_duration)?;
        dict.set_item("win_rate", m.win_rate)?;
        dict.set_item("profit_factor", m.profit_factor)?;
        dict.set_item("avg_trade_pnl", m.avg_trade_pnl)?;
        dict.set_item("exposure", m.exposure)?;
        dict.set_item("num_trades", self.inner.num_trades())?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        let m = &self.inner.metrics;
        format!(
            "BacktestResult(total_return={:?}, sharpe={:?}, max_drawdown={:?}, num_trades={})",
            m.total_return,
            m.sharpe,
            m.max_drawdown,
            self.inner.num_trades()
        )
    }
}

// ---------------------------------------------------------------------------
// run_backtest — the single FFI crossing
// ---------------------------------------------------------------------------

/// Runs a backtest. Calls `strategy.generate_signals(data)` exactly once, where
/// `data` is `{symbol: [Bar, ...]}` for every ticker, and expects a list with one
/// `{symbol: Signal | None}` dict per timestep.
#[pyfunction]
#[pyo3(signature = (strategy, data, config))]
pub fn run_backtest(
    py: Python<'_>,
    strategy: &Bound<'_, PyAny>,
    data: Vec<PyRef<'_, PyTicker>>,
    config: PyRef<'_, PyBacktestConfig>,
) -> PyResult<PyBacktestResult> {
    let tickers: Vec<Ticker> = data.iter().map(|t| t.inner.clone()).collect();
    let universe = engine::validate_universe(&tickers)?;

    let market_data = build_strategy_input(py, &tickers)?;
    let raw_signals = strategy.call_method1(intern!(py, "generate_signals"), (market_data,))?;
    let signals = extract_signal_frame(&raw_signals, &universe)?;

    let config = config.inner;
    let result = py.detach(move || engine::run(&tickers, &signals, &config))?;
    Ok(PyBacktestResult { inner: result })
}

/// `{symbol: [Bar, ...]}` for every ticker — the strategy's only view of the data.
fn build_strategy_input<'py>(py: Python<'py>, tickers: &[Ticker]) -> PyResult<Bound<'py, PyDict>> {
    let dict = PyDict::new(py);
    for ticker in tickers {
        let bars = PyList::new(py, ticker.bars().iter().map(|&inner| PyBar { inner }))?;
        dict.set_item(ticker.symbol(), bars)?;
    }
    Ok(dict)
}

/// Converts `list[dict[str, Signal | None]]` into a [`SignalFrame`] with descriptive errors.
fn extract_signal_frame(raw: &Bound<'_, PyAny>, universe: &HashSet<String>) -> PyResult<SignalFrame> {
    let list = raw.cast::<PyList>().map_err(|_| {
        PyTypeError::new_err(format!(
            "generate_signals must return a list of dicts, got {}",
            type_name(raw)
        ))
    })?;

    let rows = list
        .iter()
        .enumerate()
        .map(|(index, row)| extract_signal_row(index, &row))
        .collect::<PyResult<Vec<SignalRow>>>()?;

    Ok(SignalFrame::new(rows, universe)?)
}

fn extract_signal_row(index: usize, row: &Bound<'_, PyAny>) -> PyResult<SignalRow> {
    let dict = row.cast::<PyDict>().map_err(|_| {
        PyTypeError::new_err(format!(
            "signals at index {index} must be a dict of {{symbol: Signal | None}}, got {}",
            type_name(row)
        ))
    })?;

    let mut signals = SignalRow::with_capacity(dict.len());
    for (key, value) in dict.iter() {
        let symbol: String = key.extract().map_err(|_| {
            PyTypeError::new_err(format!("signals at index {index} have a non-str key of type {}", type_name(&key)))
        })?;
        if value.is_none() {
            continue; // None = hold
        }
        let signal = value.extract::<PyRef<'_, PySignal>>().map_err(|_| {
            PyTypeError::new_err(format!(
                "signals at index {index} for '{symbol}' must be a Signal or None, got {}",
                type_name(&value)
            ))
        })?;
        signals.insert(symbol, signal.inner);
    }
    Ok(signals)
}

fn type_name(obj: &Bound<'_, PyAny>) -> String {
    obj.get_type().name().map_or_else(|_| "<unknown>".to_string(), |n| n.to_string())
}
