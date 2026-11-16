//! Signal resolution: weight -> target dollar value -> delta units -> order.

use crate::error::{OxideError, OxideResult};
use crate::execution::Order;
use crate::strategy::Signal;

/// Signed units the portfolio should hold after acting on `signal`.
///
/// * `Long(w)`  -> `+w * equity / price`
/// * `Short(w)` -> `-w * equity / price`
/// * `Flat`     -> `0`
/// * `ScaleIn(w)` / `ScaleOut(w)` -> current ± `w * equity / price` in the current
///   direction (`ScaleOut` never crosses zero; `ScaleIn` on a flat book is a no-op).
pub fn target_quantity(
    _signal: Signal,
    _current_quantity: f64,
    _equity: f64,
    _price: f64,
) -> OxideResult<f64> {
    Err(OxideError::NotImplemented(
        "signal sizing (execution::sizing::target_quantity)",
    ))
}

/// Builds the order moving `current_quantity` to the signal's target, or `None` if no change.
pub fn resolve_order(
    _symbol: &str,
    _signal: Signal,
    _current_quantity: f64,
    _equity: f64,
    _price: f64,
    _signal_timestamp: i64,
) -> OxideResult<Option<Order>> {
    Err(OxideError::NotImplemented(
        "order resolution (execution::sizing::resolve_order)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EQUITY: f64 = 1_000.0;
    const PRICE: f64 = 10.0;

    fn target(signal: Signal, current: f64) -> f64 {
        target_quantity(signal, current, EQUITY, PRICE).unwrap()
    }

    #[test]
    fn long_and_short_target_weight_of_equity() {
        assert_eq!(target(Signal::Long(0.5), 0.0), 50.0);
        assert_eq!(target(Signal::Short(0.5), 0.0), -50.0);
        assert_eq!(target(Signal::Long(0.5), -20.0), 50.0);
    }

    #[test]
    fn flat_targets_zero() {
        assert_eq!(target(Signal::Flat, 37.0), 0.0);
    }

    #[test]
    fn flat_needs_no_valid_price_or_equity() {
        assert_eq!(target_quantity(Signal::Flat, 5.0, -10.0, 0.0).unwrap(), 0.0);
    }

    #[test]
    fn scale_in_adds_in_current_direction() {
        assert_eq!(target(Signal::ScaleIn(0.1), 20.0), 30.0);
        assert_eq!(target(Signal::ScaleIn(0.1), -20.0), -30.0);
    }

    #[test]
    fn scale_in_on_flat_book_is_noop() {
        assert_eq!(target(Signal::ScaleIn(0.1), 0.0), 0.0);
    }

    #[test]
    fn scale_out_reduces_without_crossing_zero() {
        assert_eq!(target(Signal::ScaleOut(0.1), 20.0), 10.0);
        assert_eq!(target(Signal::ScaleOut(0.1), -20.0), -10.0);
        assert_eq!(target(Signal::ScaleOut(0.1), 5.0), 0.0);
        assert_eq!(target(Signal::ScaleOut(0.1), -5.0), 0.0);
    }

    #[test]
    fn rejects_invalid_price_or_equity() {
        for (equity, price) in [(EQUITY, 0.0), (EQUITY, f64::NAN), (0.0, PRICE), (f64::INFINITY, PRICE)] {
            assert!(target_quantity(Signal::Long(0.5), 0.0, equity, price).is_err(), "{equity} {price}");
        }
    }

    #[test]
    fn resolve_order_carries_signed_delta() {
        let order = resolve_order("AAPL", Signal::Long(0.5), 20.0, EQUITY, PRICE, 7)
            .unwrap()
            .unwrap();
        assert_eq!(order.symbol, "AAPL");
        assert_eq!(order.quantity, 30.0);
        assert_eq!(order.signal_timestamp, 7);
    }

    #[test]
    fn resolve_order_sells_to_flatten() {
        let order = resolve_order("AAPL", Signal::Flat, 20.0, EQUITY, PRICE, 0)
            .unwrap()
            .unwrap();
        assert_eq!(order.quantity, -20.0);
    }

    #[test]
    fn resolve_order_is_none_when_already_at_target() {
        assert!(resolve_order("AAPL", Signal::Long(0.5), 50.0, EQUITY, PRICE, 0)
            .unwrap()
            .is_none());
        assert!(resolve_order("AAPL", Signal::Flat, 0.0, EQUITY, PRICE, 0)
            .unwrap()
            .is_none());
    }
}
