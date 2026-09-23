"""The README usage example, verbatim, as an executable spec.

Strict-xfail until the CSV parser and the engine loop are implemented.
"""

from pathlib import Path

import pandas as pd
import pytest

import oxide_engine as oxide
from conftest import CUSTOM_SCHEMA
from oxide_engine import Signal


class MACrossover:
    def generate_signals(self, data: dict) -> list[dict]:
        closes = pd.Series([b.close for b in data["AAPL"]])
        fast = closes.rolling(20).mean()
        slow = closes.rolling(50).mean()
        signals = []
        for i in range(len(closes)):
            if fast[i] > slow[i] and fast[i - 1] <= slow[i - 1]:
                signals.append({"AAPL": Signal.long(0.9)})
            elif fast[i] < slow[i] and fast[i - 1] >= slow[i - 1]:
                signals.append({"AAPL": Signal.flat()})
            else:
                signals.append({"AAPL": None})
        return signals


@pytest.mark.xfail(raises=NotImplementedError, strict=True, reason="CSV parser and engine loop are stubbed")
def test_readme_usage_example(stooq_csv: Path, custom_csv: Path):
    aapl = oxide.data_loader.from_csv(str(stooq_csv))
    msft = oxide.data_loader.from_csv(str(custom_csv), symbol="MSFT", schema=CUSTOM_SCHEMA)
    config = oxide.BacktestConfig(starting_cash=100_000, commission=0.001, slippage=0.0005)

    result = oxide.run_backtest(strategy=MACrossover(), data=[aapl, msft], config=config)

    assert isinstance(result, oxide.BacktestResult)
    assert isinstance(result.sharpe, float)
    assert 0.0 <= result.max_drawdown <= 1.0
    assert result.num_trades == len(result.trades)
    assert result.equity_curve[0] == (aapl[0].timestamp, 100_000.0)
    for trade in result.trades:
        assert isinstance(trade.pnl, float)
    frame = pd.DataFrame([t.to_dict() for t in result.trades])
    assert {"symbol", "entry_timestamp", "exit_timestamp", "pnl"} <= set(frame.columns)
