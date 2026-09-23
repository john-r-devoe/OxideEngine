//! A signed holding in one symbol.

/// `quantity > 0` is long, `< 0` is short, `== 0` is flat.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    /// Volume-weighted average entry price of the open quantity.
    pub avg_entry_price: f64,
}

impl Position {
    pub fn new(symbol: impl Into<String>, quantity: f64, avg_entry_price: f64) -> Self {
        Self { symbol: symbol.into(), quantity, avg_entry_price }
    }

    pub fn is_long(&self) -> bool {
        self.quantity > 0.0
    }

    pub fn is_short(&self) -> bool {
        self.quantity < 0.0
    }

    pub fn is_flat(&self) -> bool {
        self.quantity == 0.0
    }

    /// Signed contribution to equity at `price` (negative for shorts).
    pub fn market_value(&self, price: f64) -> f64 {
        self.quantity * price
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_follows_sign_of_quantity() {
        assert!(Position::new("A", 2.0, 1.0).is_long());
        assert!(Position::new("A", -2.0, 1.0).is_short());
        assert!(Position::new("A", 0.0, 1.0).is_flat());
    }

    #[test]
    fn short_market_value_is_negative() {
        assert_eq!(Position::new("A", -3.0, 10.0).market_value(12.0), -36.0);
    }
}
