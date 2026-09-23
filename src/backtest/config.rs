//! Backtest parameters.

use crate::error::{OxideError, OxideResult};

pub const DEFAULT_PERIODS_PER_YEAR: f64 = 252.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BacktestConfig {
    pub starting_cash: f64,
    /// Commission as a fraction of traded notional, in `[0, 1)`.
    pub commission: f64,
    /// Slippage as a fraction of price, applied against the order, in `[0, 1)`.
    pub slippage: f64,
    /// Annual risk-free rate used by Sharpe/Sortino.
    pub risk_free_rate: f64,
    /// Bars per year for annualization (252 daily, 252*390 minute, ...).
    pub periods_per_year: f64,
}

impl BacktestConfig {
    pub fn new(
        starting_cash: f64,
        commission: f64,
        slippage: f64,
        risk_free_rate: f64,
        periods_per_year: f64,
    ) -> OxideResult<Self> {
        if !starting_cash.is_finite() || starting_cash <= 0.0 {
            return Err(invalid(format!(
                "starting_cash must be finite and > 0, got {starting_cash}"
            )));
        }
        check_fraction("commission", commission)?;
        check_fraction("slippage", slippage)?;
        if !risk_free_rate.is_finite() {
            return Err(invalid(format!(
                "risk_free_rate must be finite, got {risk_free_rate}"
            )));
        }
        if !periods_per_year.is_finite() || periods_per_year <= 0.0 {
            return Err(invalid(format!(
                "periods_per_year must be finite and > 0, got {periods_per_year}"
            )));
        }
        Ok(Self {
            starting_cash,
            commission,
            slippage,
            risk_free_rate,
            periods_per_year,
        })
    }
}

fn check_fraction(name: &str, value: f64) -> OxideResult<()> {
    if value.is_finite() && (0.0..1.0).contains(&value) {
        Ok(())
    } else {
        Err(invalid(format!(
            "{name} must be a fraction in [0, 1), got {value}"
        )))
    }
}

fn invalid(message: String) -> OxideError {
    OxideError::InvalidConfig(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(cash: f64, commission: f64, slippage: f64) -> OxideResult<BacktestConfig> {
        BacktestConfig::new(cash, commission, slippage, 0.0, DEFAULT_PERIODS_PER_YEAR)
    }

    #[test]
    fn accepts_typical_config() {
        let c = config(100_000.0, 0.001, 0.0005).unwrap();
        assert_eq!(c.starting_cash, 100_000.0);
    }

    #[test]
    fn rejects_non_positive_cash() {
        assert!(config(0.0, 0.0, 0.0)
            .unwrap_err()
            .to_string()
            .contains("starting_cash"));
        assert!(config(f64::NAN, 0.0, 0.0).is_err());
    }

    #[test]
    fn rejects_cost_fractions_outside_unit_interval() {
        assert!(config(1.0, 1.0, 0.0)
            .unwrap_err()
            .to_string()
            .contains("commission"));
        assert!(config(1.0, 0.0, -0.1)
            .unwrap_err()
            .to_string()
            .contains("slippage"));
    }

    #[test]
    fn rejects_non_positive_periods_per_year() {
        let err = BacktestConfig::new(1.0, 0.0, 0.0, 0.0, 0.0).unwrap_err();
        assert!(err.to_string().contains("periods_per_year"));
    }
}
