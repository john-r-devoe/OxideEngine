//! Signal resolution: weight -> target dollar value -> delta units -> order.

use crate::error::{OxideError, OxideResult};
use crate::execution::Order;
use crate::strategy::Signal;

/// Quantity changes smaller than this fraction of the position size (or of one
/// unit, whichever is larger) are treated as float noise, not an order.
pub const ORDER_QUANTITY_TOLERANCE: f64 = 1e-9;

/// Signed units the portfolio should hold after acting on `signal`.
///
/// * `Long(w)`  -> `+w * equity / price`
/// * `Short(w)` -> `-w * equity / price`
/// * `Flat`     -> `0` (valid even when equity or price is unusable)
/// * `ScaleIn(w)` / `ScaleOut(w)` -> current ± `w * equity / price` in the current
///   direction (`ScaleOut` never crosses zero; `ScaleIn` on a flat book is a no-op).
///
/// Errors when a weighted signal is sized against non-finite or non-positive
/// equity or price.
pub fn target_quantity(
    signal: Signal,
    current_quantity: f64,
    equity: f64,
    price: f64,
) -> OxideResult<f64> {
    let Some(weight) = signal.weight() else {
        return Ok(0.0); // Flat
    };
    let units = weight * sizing_base(equity, price)?;
    let direction = current_quantity.signum();
    Ok(match signal {
        Signal::Long(_) => units,
        Signal::Short(_) => -units,
        Signal::ScaleIn(_) if current_quantity == 0.0 => 0.0,
        Signal::ScaleIn(_) => current_quantity + direction * units,
        Signal::ScaleOut(_) => direction * (current_quantity.abs() - units).max(0.0),
        Signal::Flat => 0.0,
    })
}

/// Units purchasable with the whole of `equity` at `price`.
fn sizing_base(equity: f64, price: f64) -> OxideResult<f64> {
    if !equity.is_finite() || equity <= 0.0 {
        return Err(OxideError::InvalidInput(format!(
            "cannot size a weighted signal: equity must be finite and > 0, got {equity}"
        )));
    }
    if !price.is_finite() || price <= 0.0 {
        return Err(OxideError::InvalidInput(format!(
            "cannot size a weighted signal: price must be finite and > 0, got {price}"
        )));
    }
    Ok(equity / price)
}

/// Builds the order moving `current_quantity` to the signal's target, or `None` if no change.
pub fn resolve_order(
    symbol: &str,
    signal: Signal,
    current_quantity: f64,
    equity: f64,
    price: f64,
    signal_timestamp: i64,
) -> OxideResult<Option<Order>> {
    let target = target_quantity(signal, current_quantity, equity, price)?;
    let delta = target - current_quantity;
    let scale = current_quantity.abs().max(target.abs()).max(1.0);
    if delta.abs() < ORDER_QUANTITY_TOLERANCE * scale {
        return Ok(None);
    }
    Ok(Some(Order {
        symbol: symbol.to_string(),
        quantity: delta,
        signal_timestamp,
    }))
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
        for (equity, price) in [
            (EQUITY, 0.0),
            (EQUITY, f64::NAN),
            (0.0, PRICE),
            (f64::INFINITY, PRICE),
        ] {
            assert!(
                target_quantity(Signal::Long(0.5), 0.0, equity, price).is_err(),
                "{equity} {price}"
            );
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
        assert!(
            resolve_order("AAPL", Signal::Long(0.5), 50.0, EQUITY, PRICE, 0)
                .unwrap()
                .is_none()
        );
        assert!(resolve_order("AAPL", Signal::Flat, 0.0, EQUITY, PRICE, 0)
            .unwrap()
            .is_none());
    }

    #[test]
    fn resolve_order_ignores_float_noise_relative_to_position_size() {
        // target = 0.001 * 1e12 / 1 = 1e9; current differs by float noise only
        let order = resolve_order("AAPL", Signal::Long(0.001), 1e9 + 1e-6, 1e12, 1.0, 0).unwrap();
        assert!(order.is_none(), "{order:?}");
    }
}
