"""oxide_engine.testing.assert_causal: detects look-ahead in vectorized strategies."""

import pandas as pd
import pytest

import oxide_engine as oxide
from conftest import DAY_MS, JAN_2_2024_MS, make_bar
from oxide_engine import Signal
from oxide_engine.testing import assert_causal


def zigzag(symbol: str, days: list[int]) -> "oxide.Ticker":
    """Prices that alternate direction so every signal-generating rule fires often."""
    bars = []
    for d in days:
        price = 50.0 + (d % 7) * 3.0 - (d % 3) * 2.0
        bars.append(make_bar(timestamp=JAN_2_2024_MS + d * DAY_MS, open=price, high=price + 1,
                             low=price - 1, close=price))
    return oxide.Ticker(symbol, bars)


def closes(bars) -> pd.Series:
    return pd.Series([b.close for b in bars], dtype=float)


def to_signals(up: pd.Series) -> list:
    return [Signal.long(0.5) if bool(u) else Signal.flat() for u in up]


class CausalMomentum:
    def generate_signals(self, data):
        c = closes(data["AAPL"])
        return {"AAPL": to_signals(c > c.rolling(3, min_periods=1).mean())}


class PeeksTomorrow:
    def generate_signals(self, data):
        c = closes(data["AAPL"])
        return {"AAPL": to_signals(c.shift(-1) > c)}


class CenteredMean:
    def generate_signals(self, data):
        c = closes(data["AAPL"])
        return {"AAPL": to_signals(c > c.rolling(5, center=True, min_periods=1).mean())}


class CausalLeadLag:
    """Trades MSFT on MSFT bars using AAPL closes known at or before each one."""

    def generate_signals(self, data):
        aapl = pd.Series({b.timestamp: b.close for b in data["AAPL"]}, dtype=float)
        msft_ts = [b.timestamp for b in data["MSFT"]]
        known = aapl.reindex(aapl.index.union(msft_ts)).ffill().reindex(msft_ts)
        return {"MSFT": to_signals(known.diff() > 0)}


@pytest.fixture
def aapl_only():
    return [zigzag("AAPL", list(range(30)))]


def test_causal_strategy_passes(aapl_only):
    assert_causal(CausalMomentum(), aapl_only)


def test_shift_minus_one_is_reported_with_symbol_and_index(aapl_only):
    with pytest.raises(AssertionError, match=r"look-ahead.*'AAPL' at index \d+"):
        assert_causal(PeeksTomorrow(), aapl_only)


def test_centered_window_is_reported(aapl_only):
    with pytest.raises(AssertionError, match="look-ahead"):
        assert_causal(CenteredMean(), aapl_only)


def test_cross_ticker_strategy_with_disjoint_dates_passes():
    data = [zigzag("AAPL", list(range(0, 30))), zigzag("MSFT", list(range(1, 30, 2)))]
    assert_causal(CausalLeadLag(), data, checkpoints=10)


def test_rejects_non_positive_checkpoints(aapl_only):
    with pytest.raises(ValueError, match="checkpoints"):
        assert_causal(CausalMomentum(), aapl_only, checkpoints=0)


def test_is_exposed_as_oxide_engine_testing():
    assert oxide.testing.assert_causal is assert_causal
