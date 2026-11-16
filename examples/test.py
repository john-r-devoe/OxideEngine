"""Visual sanity check: run a strategy on the AMZN sample data and plot the result.

    python examples/test.py                 # 1-minute sample, opens a window
    python examples/test.py --no-show       # only write examples/output/strategy_report.png
    python examples/test.py --fast 20 --slow 90 --short

Panels: price with moving averages and trade entries/exits, equity vs buy & hold,
drawdown, trade P&L over time with its distribution, and the metrics table.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import matplotlib.pyplot as plt
from matplotlib.figure import Figure
import pandas as pd

import oxide_engine as oxide
from oxide_engine import Signal
from oxide_engine.testing import assert_causal

EXAMPLES = Path(__file__).resolve().parent
DEFAULT_CSV = EXAMPLES / "AMZN_1min_sample.csv"
OUTPUT = EXAMPLES / "output" / "strategy_report.png"
TRADING_DAYS_PER_YEAR = 252
PERCENT_METRICS = {"total_return", "annualized_return", "annualized_volatility", "max_drawdown", "win_rate", "exposure"}


class MACrossover:
    """Go long (or short) on fast/slow moving-average crossovers."""

    def __init__(self, symbol: str, fast: int, slow: int, weight: float = 0.95, allow_short: bool = False):
        self.symbol, self.fast, self.slow = symbol, fast, slow
        self.weight, self.allow_short = weight, allow_short

    def generate_signals(self, data: dict) -> dict[str, list]:
        closes = pd.Series([b.close for b in data[self.symbol]], dtype=float)
        above = (closes.rolling(self.fast).mean() > closes.rolling(self.slow).mean()).tolist()
        signals: list = []
        for i in range(len(closes)):
            ready = i >= self.slow  # both averages existed on the previous bar
            if not ready or above[i] == above[i - 1]:
                signals.append(None)  # hold
            elif above[i]:
                signals.append(Signal.long(self.weight))
            else:
                signals.append(Signal.short(self.weight) if self.allow_short else Signal.flat())
        return {self.symbol: signals}


def periods_per_year(ticker: oxide.Ticker) -> float:
    """Annualization factor from the data's own median bars per day."""
    days = pd.Series(pd.to_datetime([b.timestamp for b in ticker.bars], unit="ms").normalize())
    return TRADING_DAYS_PER_YEAR * float(days.value_counts().median())


def to_frames(ticker: oxide.Ticker, result: oxide.BacktestResult) -> tuple[pd.DataFrame, pd.DataFrame, pd.DataFrame]:
    """Price, equity/drawdown and trade frames, each with a ``bar`` column (0..n-1).

    Plotting against bar number instead of wall-clock time removes the long flat
    lines across nights and weekends in intraday data.
    """
    timestamps = [b.timestamp for b in ticker.bars]
    bar_of = {ts: i for i, ts in enumerate(timestamps)}
    prices = pd.DataFrame(
        {"bar": range(len(timestamps)), "close": [b.close for b in ticker.bars]},
        index=pd.to_datetime(timestamps, unit="ms"),
    )
    curve = pd.DataFrame(result.equity_curve, columns=["ts", "equity"])
    curve["drawdown"] = [dd for _, dd in result.drawdown_curve]
    curve["bar"] = curve["ts"].map(bar_of)
    trades = pd.DataFrame([t.to_dict() for t in result.trades])
    if not trades.empty:
        trades["entry_bar"] = trades["entry_timestamp"].map(bar_of)
        trades["exit_bar"] = trades["exit_timestamp"].map(bar_of)
    return prices, curve, trades


def format_bar_axis(ax, prices: pd.DataFrame, max_ticks: int = 12) -> None:
    """Ticks at the first bar of each day, labelled with the date."""
    days = pd.DatetimeIndex(prices.index).normalize()
    first_bars = prices.groupby(days)["bar"].min()
    step = max(1, len(first_bars) // max_ticks + 1)
    ticks = first_bars.iloc[::step]
    ax.set_xticks(ticks.to_numpy(), [d.strftime("%b %d") for d in ticks.index], fontsize=8)
    ax.set_xlim(0, len(prices) - 1)
    ax.grid(axis="x", color="0.9")


def plot_price(ax, prices: pd.DataFrame, trades: pd.DataFrame, strategy: MACrossover) -> None:
    close, bars = prices["close"], prices["bar"]
    ax.plot(bars, close, lw=0.8, color="0.35", label="close")
    ax.plot(bars, close.rolling(strategy.fast).mean(), lw=1, label=f"MA {strategy.fast}")
    ax.plot(bars, close.rolling(strategy.slow).mean(), lw=1, label=f"MA {strategy.slow}")
    if not trades.empty:
        for side, marker, color in (("long", "^", "tab:green"), ("short", "v", "tab:purple")):
            entries = trades[trades["side"] == side]
            ax.scatter(entries["entry_bar"], entries["entry_price"], marker=marker, color=color,
                       s=40, zorder=3, label=f"{side} entry")
        ax.scatter(trades["exit_bar"], trades["exit_price"], marker="x", color="tab:red",
                   s=40, zorder=3, label="exit")
    ax.set_title(f"{strategy.symbol} price, moving averages and trades (x axis: bars, gaps removed)")
    ax.legend(loc="upper left", fontsize=8, ncol=3)
    format_bar_axis(ax, prices)


def plot_equity(ax, prices: pd.DataFrame, curve: pd.DataFrame, starting_cash: float) -> None:
    buy_hold = starting_cash * prices["close"] / prices["close"].iloc[0]
    ax.plot(curve["bar"], curve["equity"], lw=1.2, label="strategy")
    ax.plot(prices["bar"], buy_hold, lw=1, ls="--", color="0.5", label="buy & hold")
    ax.axhline(starting_cash, color="0.7", lw=0.8)
    ax.set_title("Equity")
    ax.legend(loc="upper left", fontsize=8)
    format_bar_axis(ax, prices)


def plot_drawdown(ax, prices: pd.DataFrame, curve: pd.DataFrame) -> None:
    ax.fill_between(curve["bar"], -100 * curve["drawdown"], 0, color="tab:red", alpha=0.35, lw=0)
    ax.set_title("Drawdown (%)")
    format_bar_axis(ax, prices, max_ticks=5)


def plot_trade_pnl(ax, prices: pd.DataFrame, trades: pd.DataFrame) -> None:
    ax.set_title("Cumulative realized P&L (inset: per-trade distribution)")
    format_bar_axis(ax, prices, max_ticks=5)
    if trades.empty:
        ax.text(0.5, 0.5, "no trades", ha="center", transform=ax.transAxes)
        return
    ordered = trades.sort_values("exit_bar")
    cumulative = ordered["pnl"].cumsum()
    colors = ["tab:green" if p > 0 else "tab:red" for p in ordered["pnl"]]
    ax.step(ordered["exit_bar"], cumulative, where="post", color="tab:blue")
    ax.scatter(ordered["exit_bar"], cumulative, c=colors, s=15, zorder=3)
    ax.axhline(0, color="0.7", lw=0.8)
    corner = [0.62, 0.55, 0.35, 0.4] if cumulative.iloc[-1] < 0 else [0.62, 0.06, 0.35, 0.4]
    inset = ax.inset_axes(corner)
    inset.hist(trades["pnl"], bins=min(30, max(5, len(trades))), color="tab:blue", alpha=0.7)
    inset.axvline(0, color="0.3", lw=0.8)
    inset.tick_params(labelsize=6)


def plot_metrics(ax, result: oxide.BacktestResult) -> None:
    rows = []
    for key, value in result.summary().items():
        if key in PERCENT_METRICS:
            text = f"{value:.2%}"
        elif isinstance(value, float):
            text = f"{value:,.2f}"
        else:
            text = str(value)
        rows.append([key, text])
    ax.axis("off")
    table = ax.table(cellText=rows, colLabels=["metric", "value"], loc="center", cellLoc="left")
    table.auto_set_font_size(False)
    table.set_fontsize(8)
    table.scale(1, 1.15)
    ax.set_title("Metrics")


def build_report(ticker: oxide.Ticker, result: oxide.BacktestResult, strategy: MACrossover) -> Figure:
    prices, curve, trades = to_frames(ticker, result)
    fig = plt.figure(figsize=(16, 11), constrained_layout=True)
    grid = fig.add_gridspec(3, 3)
    plot_price(fig.add_subplot(grid[0, :]), prices, trades, strategy)
    plot_equity(fig.add_subplot(grid[1, :2]), prices, curve, result.starting_cash)
    plot_metrics(fig.add_subplot(grid[1:, 2]), result)
    plot_drawdown(fig.add_subplot(grid[2, 0]), prices, curve)
    plot_trade_pnl(fig.add_subplot(grid[2, 1]), prices, trades)
    fig.suptitle(f"MA crossover {strategy.fast}/{strategy.slow} on {strategy.symbol}", fontsize=14)
    return fig


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Backtest an MA crossover on sample data and plot it.")
    parser.add_argument("--csv", type=Path, default=DEFAULT_CSV)
    parser.add_argument("--symbol", default="AMZN")
    parser.add_argument("--fast", type=int, default=30)
    parser.add_argument("--slow", type=int, default=120)
    parser.add_argument("--short", action="store_true", help="short on bearish crossovers instead of going flat")
    parser.add_argument("--no-show", action="store_true", help="save the PNG without opening a window")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    if not 0 < args.fast < args.slow:
        raise SystemExit("--fast must be positive and smaller than --slow")

    ticker = oxide.data_loader.from_csv(str(args.csv), args.symbol)
    strategy = MACrossover(args.symbol, args.fast, args.slow, allow_short=args.short)
    assert_causal(strategy, [ticker])  # fails loudly if the strategy peeks ahead

    config = oxide.BacktestConfig(
        starting_cash=100_000, commission=0.0005, slippage=0.0002, periods_per_year=periods_per_year(ticker)
    )
    result = oxide.run_backtest(strategy=strategy, data=[ticker], config=config)

    print(f"{len(ticker)} bars of {args.symbol}, {result.num_trades} trades")
    print(pd.Series(result.summary()).to_string())

    fig = build_report(ticker, result, strategy)
    OUTPUT.parent.mkdir(exist_ok=True)
    fig.savefig(OUTPUT, dpi=120)
    print(f"report saved to {OUTPUT}")
    if not args.no_show:
        plt.show()


if __name__ == "__main__":
    main()
