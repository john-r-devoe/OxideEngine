//! A single OHLCV bar in canonical form.

use crate::error::{OxideError, OxideResult};

/// One OHLCV observation. Every data source is normalized into this shape.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    /// Bar open time, Unix epoch milliseconds (UTC).
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

impl Bar {
    /// Builds a bar and enforces OHLCV consistency (see [`Bar::validate`]).
    pub fn new(
        timestamp: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> OxideResult<Self> {
        let bar = Self {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        };
        bar.validate()?;
        Ok(bar)
    }

    /// Checks the invariants every ingested bar must satisfy:
    /// finite positive prices, `low <= open, close <= high`, finite volume >= 0.
    pub fn validate(&self) -> OxideResult<()> {
        let prices = [
            ("open", self.open),
            ("high", self.high),
            ("low", self.low),
            ("close", self.close),
        ];
        if let Some((name, value)) = prices.iter().find(|(_, v)| !v.is_finite() || *v <= 0.0) {
            return Err(OxideError::InvalidBar(format!(
                "{name} must be finite and > 0, got {value}"
            )));
        }
        if !self.volume.is_finite() || self.volume < 0.0 {
            return Err(OxideError::InvalidBar(format!(
                "volume must be finite and >= 0, got {}",
                self.volume
            )));
        }
        if self.high < self.low.max(self.open).max(self.close) {
            return Err(OxideError::InvalidBar(format!(
                "high ({}) must be >= open, low and close (o={}, l={}, c={})",
                self.high, self.open, self.low, self.close
            )));
        }
        if self.low > self.open.min(self.close) {
            return Err(OxideError::InvalidBar(format!(
                "low ({}) must be <= open and close (o={}, c={})",
                self.low, self.open, self.close
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(open: f64, high: f64, low: f64, close: f64, volume: f64) -> OxideResult<Bar> {
        Bar::new(0, open, high, low, close, volume)
    }

    #[test]
    fn accepts_consistent_bar() {
        assert!(bar(10.0, 11.0, 9.0, 10.5, 100.0).is_ok());
    }

    #[test]
    fn accepts_flat_bar_where_all_prices_equal() {
        assert!(bar(10.0, 10.0, 10.0, 10.0, 0.0).is_ok());
    }

    #[test]
    fn rejects_high_below_close() {
        let err = bar(10.0, 10.2, 9.0, 10.5, 100.0).unwrap_err();
        assert!(err.to_string().contains("high"));
    }

    #[test]
    fn rejects_low_above_open() {
        let err = bar(10.0, 11.0, 10.2, 10.5, 100.0).unwrap_err();
        assert!(err.to_string().contains("low"));
    }

    #[test]
    fn rejects_non_finite_and_non_positive_prices() {
        assert!(bar(f64::NAN, 11.0, 9.0, 10.5, 100.0).is_err());
        assert!(bar(10.0, f64::INFINITY, 9.0, 10.5, 100.0).is_err());
        assert!(bar(0.0, 11.0, 0.0, 10.5, 100.0).is_err());
    }

    #[test]
    fn rejects_negative_volume() {
        let err = bar(10.0, 11.0, 9.0, 10.5, -1.0).unwrap_err();
        assert!(err.to_string().contains("volume"));
    }
}
