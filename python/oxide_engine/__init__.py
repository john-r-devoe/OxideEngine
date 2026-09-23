"""Oxide Engine: Python strategies, Rust execution.

A strategy is any object with ``generate_signals(data) -> list[dict]``. It is
called exactly once per backtest with every bar for every asset; the Rust core
then simulates orders, fills, and portfolio state without calling back.
"""

from oxide_engine import data_loader
from oxide_engine._core import (
    BacktestConfig,
    BacktestResult,
    Bar,
    Signal,
    Ticker,
    Trade,
    __version__,
    run_backtest,
)

__all__ = [
    "BacktestConfig",
    "BacktestResult",
    "Bar",
    "Signal",
    "Ticker",
    "Trade",
    "__version__",
    "data_loader",
    "run_backtest",
]
