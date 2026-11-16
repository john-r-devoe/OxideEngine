"""Test helpers for strategies.

``assert_causal`` catches look-ahead bias before a strategy goes live: a
vectorized ``generate_signals`` must give the same signal for a bar whether or
not later bars exist. A wrong ``shift``, ``center=True``, a backward fill, or
whole-history normalization all break that and are reported here.

This is a test utility. It calls ``generate_signals`` several times, which is
fine in tests and separate from ``run_backtest``'s single-call contract.
"""

from __future__ import annotations

from typing import Any, Mapping, Optional, Sequence

from oxide_engine._core import Bar, Signal, Ticker

__all__ = ["assert_causal"]

SignalLists = dict[str, list[Optional[Signal]]]


def assert_causal(strategy: Any, data: Sequence[Ticker], checkpoints: int = 5) -> None:
    """Raise ``AssertionError`` if ``strategy`` uses bars after the one it signals on.

    The strategy is run on the full data and on ``checkpoints`` truncated copies
    (every ticker cut at the same timestamp). Each truncated run's signals must
    equal the full run's signals for the same bars. Tickers with no bars before a
    cutoff are passed as empty lists.
    """
    if checkpoints < 1:
        raise ValueError(f"checkpoints must be >= 1, got {checkpoints}")
    full_data = {t.symbol: list(t.bars) for t in data}
    full = _signals(strategy, full_data)
    for cutoff in _cutoffs(full_data, checkpoints):
        truncated = {sym: [b for b in bars if b.timestamp <= cutoff] for sym, bars in full_data.items()}
        _compare(full, _signals(strategy, truncated), truncated, cutoff)


def _cutoffs(data: Mapping[str, list[Bar]], checkpoints: int) -> list[int]:
    """Up to ``checkpoints`` evenly spaced timeline timestamps, excluding the last."""
    timeline = sorted({b.timestamp for bars in data.values() for b in bars})
    if len(timeline) < 2:
        return []
    positions = {round((len(timeline) - 1) * j / (checkpoints + 1)) for j in range(1, checkpoints + 1)}
    return [timeline[p] for p in sorted(positions) if p < len(timeline) - 1]


def _signals(strategy: Any, data: Mapping[str, list[Bar]]) -> SignalLists:
    """Calls the strategy and normalizes omitted symbols to all-hold lists."""
    raw = strategy.generate_signals(dict(data))
    if not isinstance(raw, dict):
        raise TypeError(f"generate_signals must return a dict, got {type(raw).__name__}")
    result: SignalLists = {}
    for symbol, bars in data.items():
        entries = list(raw.get(symbol, [None] * len(bars)))
        if len(entries) != len(bars):
            raise ValueError(f"signals for '{symbol}' have {len(entries)} entries but {len(bars)} bars")
        result[symbol] = entries
    return result


def _compare(full: SignalLists, partial: SignalLists, data: Mapping[str, list[Bar]], cutoff: int) -> None:
    for symbol, entries in partial.items():
        for index, (seen_later, seen_now) in enumerate(zip(full[symbol], entries)):
            if seen_later != seen_now:
                timestamp = data[symbol][index].timestamp
                raise AssertionError(
                    f"look-ahead detected: signal for '{symbol}' at index {index} "
                    f"(timestamp {timestamp}) is {seen_now!r} when data ends at {cutoff} "
                    f"but {seen_later!r} with the full history"
                )
