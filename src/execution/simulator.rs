//! Fill simulation with proportional commission and slippage.
//!
//! Fill-timing rule (avoids look-ahead): a signal emitted on bar `t` is filled
//! at the *open* of the next bar `t+1` for that symbol.

use crate::backtest::BacktestConfig;
use crate::data::Bar;
use crate::error::{OxideError, OxideResult};
use crate::execution::{Fill, Order};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FillSimulator {
    /// Fraction of traded notional charged as commission.
    pub commission: f64,
    /// Fraction by which the fill price moves against the order.
    pub slippage: f64,
}

impl FillSimulator {
    pub fn from_config(config: &BacktestConfig) -> Self {
        Self { commission: config.commission, slippage: config.slippage }
    }

    /// Fills `order` against `bar`:
    /// price = `open × (1 ± slippage)`, commission = `|qty| × price × commission`.
    pub fn fill(&self, _order: &Order, _bar: &Bar) -> OxideResult<Fill> {
        Err(OxideError::NotImplemented("fill simulation (execution::simulator::FillSimulator::fill)"))
    }
}
