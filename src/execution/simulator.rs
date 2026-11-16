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
        Self {
            commission: config.commission,
            slippage: config.slippage,
        }
    }

    /// Fills `order` against `bar`:
    /// price = `open × (1 ± slippage)`, commission = `|qty| × price × commission`.
    pub fn fill(&self, order: &Order, bar: &Bar) -> OxideResult<Fill> {
        self.fill_at_price(order, bar.open, bar.timestamp)
    }

    /// Fills `order` at `reference_price` (e.g. the final close when liquidating),
    /// applying the same flat slippage and commission as [`Self::fill`].
    pub fn fill_at_price(
        &self,
        order: &Order,
        reference_price: f64,
        timestamp: i64,
    ) -> OxideResult<Fill> {
        if !order.quantity.is_finite() || order.quantity == 0.0 {
            return Err(OxideError::InvalidInput(format!(
                "order for '{}' must have a finite non-zero quantity, got {}",
                order.symbol, order.quantity
            )));
        }
        let direction = if order.is_buy() { 1.0 } else { -1.0 };
        let price = reference_price * (1.0 + direction * self.slippage);
        Ok(Fill {
            symbol: order.symbol.clone(),
            timestamp,
            quantity: order.quantity,
            price,
            commission: order.quantity.abs() * price * self.commission,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    fn sim() -> FillSimulator {
        FillSimulator {
            commission: 0.001,
            slippage: 0.01,
        }
    }

    fn bar() -> Bar {
        Bar::new(42, 100.0, 110.0, 90.0, 105.0, 1_000.0).unwrap()
    }

    fn order(quantity: f64) -> Order {
        Order {
            symbol: "AAPL".into(),
            quantity,
            signal_timestamp: 41,
        }
    }

    #[test]
    fn buy_fills_above_open_with_commission() {
        let fill = sim().fill(&order(10.0), &bar()).unwrap();
        assert!((fill.price - 101.0).abs() < EPS);
        assert!((fill.commission - 10.0 * 101.0 * 0.001).abs() < EPS);
        assert_eq!(fill.quantity, 10.0);
        assert_eq!(fill.timestamp, 42);
        assert_eq!(fill.symbol, "AAPL");
    }

    #[test]
    fn sell_fills_below_open_with_commission_on_abs_quantity() {
        let fill = sim().fill(&order(-10.0), &bar()).unwrap();
        assert!((fill.price - 99.0).abs() < EPS);
        assert!((fill.commission - 10.0 * 99.0 * 0.001).abs() < EPS);
        assert_eq!(fill.quantity, -10.0);
    }

    #[test]
    fn zero_costs_fill_at_open() {
        let free = FillSimulator {
            commission: 0.0,
            slippage: 0.0,
        };
        let fill = free.fill(&order(3.0), &bar()).unwrap();
        assert_eq!(fill.price, 100.0);
        assert_eq!(fill.commission, 0.0);
    }

    #[test]
    fn fill_at_price_applies_same_costs_to_given_price() {
        let fill = sim().fill_at_price(&order(-2.0), 200.0, 9).unwrap();
        assert!((fill.price - 198.0).abs() < EPS);
        assert!((fill.commission - 2.0 * 198.0 * 0.001).abs() < EPS);
        assert_eq!(fill.timestamp, 9);
    }

    #[test]
    fn rejects_zero_or_non_finite_quantity() {
        assert!(sim().fill(&order(0.0), &bar()).is_err());
        assert!(sim().fill(&order(f64::NAN), &bar()).is_err());
    }
}
