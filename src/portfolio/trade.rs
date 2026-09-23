//! A completed round-trip trade — the unit of per-trade analysis in Python.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeSide {
    Long,
    Short,
}

impl TradeSide {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }
}

/// One closed (or partially closed) position. Positions still open at the end
/// of a backtest are closed at the final bar's close.
#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub symbol: String,
    pub side: TradeSide,
    /// Absolute number of units closed.
    pub quantity: f64,
    pub entry_timestamp: i64,
    pub exit_timestamp: i64,
    /// Average fill price (after slippage) of the entry.
    pub entry_price: f64,
    /// Average fill price (after slippage) of the exit.
    pub exit_price: f64,
    /// Net profit in currency after commissions.
    pub pnl: f64,
    /// `pnl / (quantity × entry_price)`.
    pub return_pct: f64,
    /// Total commission paid on entry and exit.
    pub commission: f64,
    /// Number of timeline steps the position was held.
    pub bars_held: usize,
}
