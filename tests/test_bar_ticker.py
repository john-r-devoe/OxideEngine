"""Bar and Ticker: canonical market data containers."""

import math

import pytest

import oxide_engine as oxide
from conftest import JAN_2_2024_MS, make_bar


def test_bar_exposes_canonical_fields():
    bar = make_bar()

    assert bar.timestamp == JAN_2_2024_MS
    assert (bar.open, bar.high, bar.low, bar.close, bar.volume) == (10.0, 11.0, 9.5, 10.5, 1_000.0)


@pytest.mark.parametrize(
    "overrides",
    [
        {"high": 9.0},  # high below low
        {"high": 10.2},  # high below close
        {"low": 10.8},  # low above open
        {"open": 0.0, "low": 0.0},  # non-positive price
        {"close": math.nan},
        {"volume": -1.0},
    ],
)
def test_bar_rejects_inconsistent_ohlcv(overrides):
    with pytest.raises(ValueError):
        make_bar(**overrides)


def test_ticker_supports_len_and_indexing(aapl):
    assert aapl.symbol == "AAPL"
    assert len(aapl) == 5
    assert aapl[0].open == 10.0
    assert aapl[-1].open == 14.0


def test_ticker_index_out_of_range_raises_index_error(aapl):
    with pytest.raises(IndexError):
        aapl[5]
    with pytest.raises(IndexError):
        aapl[-6]


def test_ticker_bars_returns_list_of_bar(aapl):
    bars = aapl.bars

    assert isinstance(bars, list)
    assert all(isinstance(b, oxide.Bar) for b in bars)


@pytest.mark.parametrize("symbol", ["", "   "])
def test_ticker_rejects_blank_symbol(symbol):
    with pytest.raises(ValueError, match="symbol"):
        oxide.Ticker(symbol, [make_bar()])


def test_ticker_rejects_empty_bars():
    with pytest.raises(ValueError, match="bars"):
        oxide.Ticker("AAPL", [])
