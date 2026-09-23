"""Shared fixtures for the Oxide Engine Python test suite."""

from __future__ import annotations

from pathlib import Path

import pytest

import oxide_engine as oxide

DAY_MS = 86_400_000
JAN_2_2024_MS = 1_704_153_600_000  # 2024-01-02T00:00:00Z


def make_bar(
    timestamp: int = JAN_2_2024_MS,
    open: float = 10.0,
    high: float = 11.0,
    low: float = 9.5,
    close: float = 10.5,
    volume: float = 1_000.0,
) -> "oxide.Bar":
    return oxide.Bar(timestamp, open, high, low, close, volume)


def make_ticker(symbol: str = "AAPL", n_bars: int = 5, start_price: float = 10.0) -> "oxide.Ticker":
    bars = [
        make_bar(
            timestamp=JAN_2_2024_MS + i * DAY_MS,
            open=start_price + i,
            high=start_price + i + 1.0,
            low=start_price + i - 0.5,
            close=start_price + i + 0.5,
        )
        for i in range(n_bars)
    ]
    return oxide.Ticker(symbol, bars)


@pytest.fixture
def aapl() -> "oxide.Ticker":
    return make_ticker("AAPL")


@pytest.fixture
def msft() -> "oxide.Ticker":
    return make_ticker("MSFT", start_price=100.0)


@pytest.fixture
def config() -> "oxide.BacktestConfig":
    return oxide.BacktestConfig(starting_cash=100_000, commission=0.001, slippage=0.0005)


@pytest.fixture
def stooq_csv(tmp_path: Path) -> Path:
    """Synthetic file in Stooq's angle-bracket header format (60 daily bars)."""
    path = tmp_path / "AAPL.us.txt"
    rows = ["<TICKER>,<PER>,<DATE>,<TIME>,<OPEN>,<HIGH>,<LOW>,<CLOSE>,<VOL>,<OPENINT>"]
    for i in range(60):
        price = 100.0 + i
        date = f"2024{1 + i // 28:02d}{1 + i % 28:02d}"
        rows.append(f"AAPL.US,D,{date},000000,{price},{price + 2},{price - 1},{price + 1},{1000 + i},0")
    path.write_text("\n".join(rows) + "\n")
    return path


@pytest.fixture
def custom_csv(tmp_path: Path) -> Path:
    """Synthetic file with user-specific headers, loaded through a schema."""
    path = tmp_path / "data.csv"
    path.write_text(
        "Date,Open,High,Low,Close,Vol\n"
        "2024-01-02,10.0,11.0,9.5,10.5,1000\n"
        "2024-01-03,10.5,11.5,10.0,11.0,1200\n"
    )
    return path


CUSTOM_SCHEMA = {
    "Date": "timestamp",
    "Open": "open",
    "High": "high",
    "Low": "low",
    "Close": "close",
    "Vol": "volume",
}
