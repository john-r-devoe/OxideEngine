"""Moving-average crossover — the canonical Oxide Engine usage example.

Put your CSVs in ./data (git-ignored) and run:  python examples/ma_crossover.py
"""

import pandas as pd

import oxide_engine as oxide
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


def main() -> None:
    aapl = oxide.data_loader.from_csv("data/AAPL.us.txt")
    msft = oxide.data_loader.from_csv(
        "data/data.csv",
        symbol="MSFT",
        schema={"Date": "timestamp", "Open": "open", "High": "high", "Low": "low", "Close": "close", "Vol": "volume"},
    )

    config = oxide.BacktestConfig(starting_cash=100_000, commission=0.001, slippage=0.0005)
    result = oxide.run_backtest(strategy=MACrossover(), data=[aapl, msft], config=config)

    print(pd.Series(result.summary()))

    equity = pd.DataFrame(result.equity_curve, columns=["timestamp", "equity"])
    equity["timestamp"] = pd.to_datetime(equity["timestamp"], unit="ms")
    trades = pd.DataFrame([t.to_dict() for t in result.trades])
    print(equity.tail())
    print(trades.head())


if __name__ == "__main__":
    main()
