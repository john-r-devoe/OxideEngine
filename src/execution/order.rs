//! A market order generated from a resolved signal.

/// Signed market order: `quantity > 0` buys, `< 0` sells.
#[derive(Debug, Clone, PartialEq)]
pub struct Order {
    pub symbol: String,
    pub quantity: f64,
    /// Timestamp of the bar whose signal produced this order.
    pub signal_timestamp: i64,
}

impl Order {
    pub fn is_buy(&self) -> bool {
        self.quantity > 0.0
    }
}
