//! Portfolio state: cash plus signed positions.
//!
//! Universal invariant: `equity = cash + Σ(quantity × current_price)`.
//! Shorts are simply negative quantities — there is no separate debt ledger.

pub mod position;
pub mod trade;

use std::collections::HashMap;

pub use position::Position;
pub use trade::{Trade, TradeSide};

use crate::error::{OxideError, OxideResult};
use crate::execution::Fill;

#[derive(Debug, Clone, PartialEq)]
pub struct Portfolio {
    cash: f64,
    positions: HashMap<String, Position>,
}

impl Portfolio {
    pub fn new(starting_cash: f64) -> Self {
        Self { cash: starting_cash, positions: HashMap::new() }
    }

    pub fn cash(&self) -> f64 {
        self.cash
    }

    pub fn position(&self, symbol: &str) -> Option<&Position> {
        self.positions.get(symbol)
    }

    /// Signed quantity held for `symbol` (0.0 when flat or never traded).
    pub fn quantity(&self, symbol: &str) -> f64 {
        self.positions.get(symbol).map_or(0.0, |p| p.quantity)
    }

    /// Marks every open position to `prices`. Errors if a held symbol has no price.
    pub fn equity(&self, prices: &HashMap<String, f64>) -> OxideResult<f64> {
        self.positions.values().try_fold(self.cash, |acc, position| {
            let price = prices.get(&position.symbol).ok_or_else(|| {
                OxideError::InvalidInput(format!("no mark price for held symbol '{}'", position.symbol))
            })?;
            Ok(acc + position.market_value(*price))
        })
    }

    /// Applies a fill: moves cash (incl. commission), updates the signed position and
    /// its average entry price, and returns a [`Trade`] when a position is reduced,
    /// closed, or flipped.
    pub fn apply_fill(&mut self, _fill: &Fill) -> OxideResult<Option<Trade>> {
        Err(OxideError::NotImplemented("fill accounting (portfolio::Portfolio::apply_fill)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_portfolio_equity_equals_cash() {
        let portfolio = Portfolio::new(1_000.0);
        assert_eq!(portfolio.equity(&HashMap::new()).unwrap(), 1_000.0);
        assert_eq!(portfolio.quantity("AAPL"), 0.0);
    }

    #[test]
    fn equity_is_cash_plus_signed_market_value() {
        let mut portfolio = Portfolio::new(1_000.0);
        portfolio.positions.insert("L".into(), Position::new("L", 10.0, 5.0));
        portfolio.positions.insert("S".into(), Position::new("S", -4.0, 20.0));
        let prices = HashMap::from([("L".to_string(), 6.0), ("S".to_string(), 25.0)]);

        // 1000 + 10*6 + (-4)*25
        assert_eq!(portfolio.equity(&prices).unwrap(), 960.0);
    }

    #[test]
    fn equity_errors_when_held_symbol_has_no_price() {
        let mut portfolio = Portfolio::new(1_000.0);
        portfolio.positions.insert("L".into(), Position::new("L", 1.0, 5.0));
        assert!(portfolio.equity(&HashMap::new()).is_err());
    }
}
