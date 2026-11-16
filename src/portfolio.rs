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
        Self {
            cash: starting_cash,
            positions: HashMap::new(),
        }
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
        self.positions
            .values()
            .try_fold(self.cash, |acc, position| {
                let price = prices.get(&position.symbol).ok_or_else(|| {
                    OxideError::InvalidInput(format!(
                        "no mark price for held symbol '{}'",
                        position.symbol
                    ))
                })?;
                Ok(acc + position.market_value(*price))
            })
    }

    /// Applies a fill: moves cash (incl. commission), updates the signed position and
    /// its average entry price, and returns a [`Trade`] when a position is reduced,
    /// closed, or flipped.
    pub fn apply_fill(&mut self, _fill: &Fill) -> OxideResult<Option<Trade>> {
        Err(OxideError::NotImplemented(
            "fill accounting (portfolio::Portfolio::apply_fill)",
        ))
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
        portfolio
            .positions
            .insert("L".into(), Position::new("L", 10.0, 5.0));
        portfolio
            .positions
            .insert("S".into(), Position::new("S", -4.0, 20.0));
        let prices = HashMap::from([("L".to_string(), 6.0), ("S".to_string(), 25.0)]);

        // 1000 + 10*6 + (-4)*25
        assert_eq!(portfolio.equity(&prices).unwrap(), 960.0);
    }

    #[test]
    fn equity_errors_when_held_symbol_has_no_price() {
        let mut portfolio = Portfolio::new(1_000.0);
        portfolio
            .positions
            .insert("L".into(), Position::new("L", 1.0, 5.0));
        assert!(portfolio.equity(&HashMap::new()).is_err());
    }

    const EPS: f64 = 1e-9;

    fn fill(quantity: f64, price: f64, commission: f64, timestamp: i64) -> Fill {
        Fill {
            symbol: "A".into(),
            timestamp,
            quantity,
            price,
            commission,
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn opening_fill_moves_cash_and_creates_position() {
        let mut p = Portfolio::new(1_000.0);
        let trade = p.apply_fill(&fill(10.0, 10.0, 1.0, 100), 0).unwrap();
        assert!(trade.is_none());
        assert!(close(p.cash(), 1_000.0 - 100.0 - 1.0));
        let pos = p.position("A").unwrap();
        assert_eq!(pos.quantity, 10.0);
        assert_eq!(pos.avg_entry_price, 10.0);
        assert_eq!(pos.entry_timestamp, 100);
        assert_eq!(pos.entry_step, 0);
    }

    #[test]
    fn adding_to_position_uses_volume_weighted_entry_and_keeps_entry_time() {
        let mut p = Portfolio::new(1_000.0);
        p.apply_fill(&fill(10.0, 10.0, 1.0, 100), 0).unwrap();
        p.apply_fill(&fill(30.0, 14.0, 2.0, 200), 3).unwrap();
        let pos = p.position("A").unwrap();
        assert_eq!(pos.quantity, 40.0);
        assert!(close(pos.avg_entry_price, (100.0 + 420.0) / 40.0));
        assert_eq!(pos.entry_timestamp, 100);
        assert!(close(pos.entry_commission, 3.0));
    }

    #[test]
    fn full_close_emits_long_trade_with_net_pnl() {
        let mut p = Portfolio::new(1_000.0);
        p.apply_fill(&fill(10.0, 10.0, 1.0, 100), 2).unwrap();
        let t = p.apply_fill(&fill(-10.0, 12.0, 1.2, 500), 7).unwrap().unwrap();
        assert_eq!(t.side, TradeSide::Long);
        assert_eq!(t.quantity, 10.0);
        assert_eq!((t.entry_timestamp, t.exit_timestamp), (100, 500));
        assert_eq!((t.entry_price, t.exit_price), (10.0, 12.0));
        assert!(close(t.commission, 2.2));
        assert!(close(t.pnl, 20.0 - 2.2));
        assert!(close(t.return_pct, (20.0 - 2.2) / 100.0));
        assert_eq!(t.bars_held, 5);
        assert!(p.position("A").is_none());
        assert!(close(p.cash(), 1_000.0 + 20.0 - 2.2));
    }

    #[test]
    fn partial_close_allocates_entry_commission_pro_rata() {
        let mut p = Portfolio::new(1_000.0);
        p.apply_fill(&fill(10.0, 10.0, 2.0, 100), 0).unwrap();
        let t = p.apply_fill(&fill(-4.0, 11.0, 0.4, 200), 1).unwrap().unwrap();
        assert_eq!(t.quantity, 4.0);
        assert!(close(t.commission, 0.8 + 0.4));
        assert!(close(t.pnl, 4.0 - 1.2));
        let pos = p.position("A").unwrap();
        assert_eq!(pos.quantity, 6.0);
        assert_eq!(pos.avg_entry_price, 10.0);
        assert!(close(pos.entry_commission, 1.2));
    }

    #[test]
    fn flip_closes_old_side_and_opens_remainder_at_fill() {
        let mut p = Portfolio::new(1_000.0);
        p.apply_fill(&fill(10.0, 10.0, 1.0, 100), 0).unwrap();
        let t = p.apply_fill(&fill(-15.0, 12.0, 1.5, 300), 4).unwrap().unwrap();
        assert_eq!((t.side, t.quantity), (TradeSide::Long, 10.0));
        assert!(close(t.commission, 1.0 + 1.0));
        let pos = p.position("A").unwrap();
        assert_eq!(pos.quantity, -5.0);
        assert_eq!(pos.avg_entry_price, 12.0);
        assert_eq!((pos.entry_timestamp, pos.entry_step), (300, 4));
        assert!(close(pos.entry_commission, 0.5));
    }

    #[test]
    fn short_round_trip_profits_when_price_falls() {
        let mut p = Portfolio::new(1_000.0);
        p.apply_fill(&fill(-10.0, 10.0, 0.0, 100), 0).unwrap();
        assert!(close(p.cash(), 1_100.0));
        let t = p.apply_fill(&fill(10.0, 8.0, 0.0, 200), 2).unwrap().unwrap();
        assert_eq!(t.side, TradeSide::Short);
        assert!(close(t.pnl, 20.0));
        assert!(close(t.return_pct, 0.2));
        assert!(close(p.cash(), 1_020.0));
    }

    #[test]
    fn equity_invariant_holds_after_fills() {
        let mut p = Portfolio::new(1_000.0);
        p.apply_fill(&fill(10.0, 10.0, 1.0, 1), 0).unwrap();
        p.apply_fill(&fill(-25.0, 11.0, 2.0, 2), 1).unwrap();
        let prices = HashMap::from([("A".to_string(), 9.0)]);
        let expected = p.cash() + p.quantity("A") * 9.0;
        assert!(close(p.equity(&prices).unwrap(), expected));
        // realized 10*(11-10) - (1 + 2*10/25); open short 15 @ 11 marked at 9 => +30;
        // open commission 2*15/25 already paid from cash.
        assert!(close(expected, 1_000.0 + 10.0 - 1.8 + 30.0 - 1.2));
    }

    #[test]
    fn rejects_invalid_fill() {
        let mut p = Portfolio::new(1_000.0);
        for bad in [
            fill(0.0, 10.0, 0.0, 0),
            fill(1.0, 0.0, 0.0, 0),
            fill(1.0, f64::NAN, 0.0, 0),
            fill(1.0, 10.0, -1.0, 0),
        ] {
            assert!(p.apply_fill(&bad, 0).is_err(), "{bad:?}");
        }
        assert_eq!(p.cash(), 1_000.0);
    }
}
