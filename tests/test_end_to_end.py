"""The README usage example, verbatim, as an executable spec, plus
multi-ticker alignment acceptance tests for the per-ticker signal contract.
"""

import math
from pathlib import Path

import pandas as pd
import pytest

import oxide_engine as oxide
from conftest import CUSTOM_SCHEMA, DAY_MS, JAN_2_2024_MS, make_bar
from oxide_engine import Signal


class MACrossover:
    def generate_signals(self, data: dict) -> dict[str, list]:
        closes = pd.Series([b.close for b in data["AAPL"]])
        fast = closes.rolling(20).mean()
        slow = closes.rolling(50).mean()
        signals = []
        for i in range(len(closes)):
            if fast[i] > slow[i] and fast[i - 1] <= slow[i - 1]:
                signals.append(Signal.long(0.9))
            elif fast[i] < slow[i] and fast[i - 1] >= slow[i - 1]:
                signals.append(Signal.flat())
            else:
                signals.append(None)
        return {"AAPL": signals}  # MSFT omitted = hold


@pytest.fixture
def crossover_csv(tmp_path: Path) -> Path:
    """Stooq-format AAPL file whose closes oscillate, so the MAs cross."""
    path = tmp_path / "AAPL.us.txt"
    rows = ["<TICKER>,<PER>,<DATE>,<TIME>,<OPEN>,<HIGH>,<LOW>,<CLOSE>,<VOL>,<OPENINT>"]
    for i in range(140):
        price = 100.0 + 20.0 * math.sin(2 * math.pi * i / 60)
        date = f"2024{1 + i // 28:02d}{1 + i % 28:02d}"
        rows.append(f"AAPL.US,D,{date},000000,{price},{price + 2},{price - 2},{price + 1},1000,0")
    path.write_text("\n".join(rows) + "\n")
    return path


def test_readme_usage_example(crossover_csv: Path, custom_csv: Path):
    aapl = oxide.data_loader.from_csv(str(crossover_csv))
    msft = oxide.data_loader.from_csv(str(custom_csv), symbol="MSFT", schema=CUSTOM_SCHEMA)
    config = oxide.BacktestConfig(starting_cash=100_000, commission=0.001, slippage=0.0005)

    result = oxide.run_backtest(strategy=MACrossover(), data=[aapl, msft], config=config)

    assert isinstance(result, oxide.BacktestResult)
    assert isinstance(result.sharpe, float)
    assert 0.0 <= result.max_drawdown <= 1.0
    assert result.num_trades == len(result.trades) >= 1
    assert result.equity_curve[0] == (aapl[0].timestamp, 100_000.0)
    for trade in result.trades:
        assert isinstance(trade.pnl, float)
    frame = pd.DataFrame([t.to_dict() for t in result.trades])
    assert {"symbol", "entry_timestamp", "exit_timestamp", "pnl"} <= set(frame.columns)


# --- per-ticker alignment ------------------------------------------------------


def _ticker(symbol: str, days: list[int], start_price: float) -> "oxide.Ticker":
    bars = [
        make_bar(timestamp=JAN_2_2024_MS + d * DAY_MS, open=start_price + d, high=start_price + d + 1,
                 low=start_price + d - 1, close=start_price + d)
        for d in days
    ]
    return oxide.Ticker(symbol, bars)


class FixedSignals:
    def __init__(self, signals):
        self.signals = signals

    def generate_signals(self, data):
        return {sym: [self.signals.get(sym, {}).get(i) for i in range(len(bars))] for sym, bars in data.items()}


def test_tickers_with_disjoint_dates_fill_on_their_own_next_bar():
    # AAPL lacks day 1; MSFT lacks day 3. Each symbol's signals align to its own bars.
    aapl = _ticker("AAPL", [0, 2, 3, 4], 10.0)
    msft = _ticker("MSFT", [0, 1, 2, 4], 100.0)
    strategy = FixedSignals({"MSFT": {0: Signal.long(0.25)}, "AAPL": {1: Signal.long(0.25)}})
    config = oxide.BacktestConfig(starting_cash=10_000)

    result = oxide.run_backtest(strategy=strategy, data=[aapl, msft], config=config)

    entries = {t.symbol: (t.entry_timestamp, t.entry_price) for t in result.trades}
    assert entries["MSFT"] == (msft[1].timestamp, msft[1].open)  # day 1
    assert entries["AAPL"] == (aapl[2].timestamp, aapl[2].open)  # day 3, not day 2
    assert [ts for ts, _ in result.equity_curve] == [JAN_2_2024_MS + d * DAY_MS for d in range(5)]


def test_signal_on_symbols_last_bar_is_dropped_when_other_tickers_continue():
    aapl = _ticker("AAPL", [0, 1, 2, 3], 10.0)
    msft = _ticker("MSFT", [0, 1], 100.0)
    strategy = FixedSignals({"MSFT": {1: Signal.long(0.5)}})

    result = oxide.run_backtest(strategy=strategy, data=[aapl, msft], config=oxide.BacktestConfig(10_000))

    assert result.num_trades == 0
    assert result.final_equity == 10_000


class LeadLag:
    """Cross-ticker example: trade MSFT only on MSFT bars, using the latest AAPL
    close known at or before each MSFT timestamp (no look-ahead)."""

    def generate_signals(self, data):
        aapl = pd.Series({b.timestamp: b.close for b in data["AAPL"]})
        msft_ts = [b.timestamp for b in data["MSFT"]]
        known = aapl.reindex(aapl.index.union(msft_ts)).ffill().loc[msft_ts]
        rising = (known.diff() > 0).tolist()
        signals = []
        for i, up in enumerate(rising):
            was_up = i > 0 and rising[i - 1]
            if up and not was_up:
                signals.append(Signal.long(0.5))
            elif was_up and not up:
                signals.append(Signal.flat())
            else:
                signals.append(None)  # hold: only act on transitions
        return {"MSFT": signals}


def test_cross_ticker_strategy_reacts_only_on_the_later_tickers_bars():
    aapl = _ticker("AAPL", [0, 1, 2, 3, 4, 5], 10.0)
    msft = _ticker("MSFT", [0, 2, 4, 5], 100.0)

    result = oxide.run_backtest(strategy=LeadLag(), data=[aapl, msft], config=oxide.BacktestConfig(10_000))

    msft_trades = [t for t in result.trades if t.symbol == "MSFT"]
    assert len(msft_trades) == 1
    assert msft_trades[0].entry_timestamp == msft[2].timestamp  # signal on MSFT bar 1 (day 2)
