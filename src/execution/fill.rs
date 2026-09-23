//! An executed order.

#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    pub symbol: String,
    pub timestamp: i64,
    /// Signed executed quantity (`> 0` bought, `< 0` sold).
    pub quantity: f64,
    /// Execution price including slippage.
    pub price: f64,
    /// Commission charged in currency.
    pub commission: f64,
}
