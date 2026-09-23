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
