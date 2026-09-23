//! Trading intent expressed as a fraction of portfolio equity.
//!
//! Python only states *what exposure it wants*; Rust resolves
//! weight -> target dollar value -> delta units -> order -> fill
//! (see [`crate::execution::sizing`]).

use crate::error::{OxideError, OxideResult};

/// Target-exposure instruction for one symbol at one timestep.
/// Weights are fractions of total portfolio equity in `[0.0, 1.0]`.
/// "Hold" is expressed by the *absence* of a signal (Python `None`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal {
    /// Target a long position worth `weight * equity`.
    Long(f64),
    /// Target a short position worth `weight * equity` (negative quantity).
    Short(f64),
    /// Close any open position.
    Flat,
    /// Increase current exposure by `weight * equity` in its current direction.
    ScaleIn(f64),
    /// Reduce current exposure by `weight * equity`, never flipping direction.
    ScaleOut(f64),
}

impl Signal {
    pub fn long(weight: f64) -> OxideResult<Self> {
        validate_weight(weight).map(Self::Long)
    }

    pub fn short(weight: f64) -> OxideResult<Self> {
        validate_weight(weight).map(Self::Short)
    }

    pub fn flat() -> Self {
        Self::Flat
    }

    pub fn scale_in(weight: f64) -> OxideResult<Self> {
        validate_weight(weight).map(Self::ScaleIn)
    }

    pub fn scale_out(weight: f64) -> OxideResult<Self> {
        validate_weight(weight).map(Self::ScaleOut)
    }

    /// Lowercase name matching the Python factory (`"long"`, `"scale_in"`, ...).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Long(_) => "long",
            Self::Short(_) => "short",
            Self::Flat => "flat",
            Self::ScaleIn(_) => "scale_in",
            Self::ScaleOut(_) => "scale_out",
        }
    }

    pub fn weight(&self) -> Option<f64> {
        match *self {
            Self::Long(w) | Self::Short(w) | Self::ScaleIn(w) | Self::ScaleOut(w) => Some(w),
            Self::Flat => None,
        }
    }
}

fn validate_weight(weight: f64) -> OxideResult<f64> {
    if weight.is_finite() && (0.0..=1.0).contains(&weight) {
        Ok(weight)
    } else {
        Err(OxideError::InvalidSignal(format!(
            "weight must be within [0.0, 1.0], got {weight}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_constructors_keep_weight_and_kind() {
        let cases = [
            (Signal::long(0.5).unwrap(), "long"),
            (Signal::short(0.5).unwrap(), "short"),
            (Signal::scale_in(0.5).unwrap(), "scale_in"),
            (Signal::scale_out(0.5).unwrap(), "scale_out"),
        ];
        for (signal, kind) in cases {
            assert_eq!(signal.kind(), kind);
            assert_eq!(signal.weight(), Some(0.5));
        }
    }

    #[test]
    fn flat_has_no_weight() {
        assert_eq!(Signal::flat().weight(), None);
        assert_eq!(Signal::flat().kind(), "flat");
    }

    #[test]
    fn accepts_inclusive_bounds() {
        assert!(Signal::long(0.0).is_ok());
        assert!(Signal::long(1.0).is_ok());
    }

    #[test]
    fn rejects_out_of_range_or_non_finite_weight() {
        for w in [-0.01, 1.01, f64::NAN, f64::INFINITY] {
            let err = Signal::short(w).unwrap_err();
            assert!(err.to_string().contains("weight"), "{w}");
        }
    }
}
