"""Oxide Engine: Python strategies, Rust execution.

A strategy is any object with
``generate_signals(data: dict[str, list[Bar]]) -> dict[str, list[Signal | None]]``,
returning one entry per bar of each symbol (omitted symbols hold). It is called
exactly once per backtest with every bar for every asset; the Rust core then
simulates orders, fills, and portfolio state without calling back. A signal on
bar ``i`` of a symbol fills at the open of that symbol's bar ``i + 1``.
"""

from oxide_engine import data_loader, testing
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
    "testing",
]
