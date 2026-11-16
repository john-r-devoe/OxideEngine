# Oxide Engine

A backtesting engine where **strategies are written in Python** and **execution runs in Rust**.

Your strategy is called **once** with every bar for every asset and returns one
signal per bar for each asset. Rust then turns those weights into orders, fills,
portfolio state, and an equity curve, without calling back into Python.

> **Status:** CSV loading, sizing, fill simulation, portfolio accounting,
> metrics, and the event loop are implemented. Costs are a flat commission and a
> flat slippage percentage from `BacktestConfig`.

## Usage

```python
import oxide_engine as oxide
import pandas as pd
from oxide_engine import Signal

aapl = oxide.data_loader.from_csv("AAPL.us.txt")                 # headers auto-detected
msft = oxide.data_loader.from_csv("data.csv", schema={           # or map your own headers
    "Date": "timestamp", "Open": "open", "High": "high",
    "Low": "low", "Close": "close", "Vol": "volume",
})

class MACrossover:
    def generate_signals(self, data: dict) -> dict[str, list]:
        closes = pd.Series([b.close for b in data["AAPL"]])
        fast = closes.rolling(20).mean()
        slow = closes.rolling(50).mean()
        signals = []
        for i in range(len(closes)):            # one entry per AAPL bar
            if fast[i] > slow[i] and fast[i-1] <= slow[i-1]:
                signals.append(Signal.long(0.9))
            elif fast[i] < slow[i] and fast[i-1] >= slow[i-1]:
                signals.append(Signal.flat())
            else:
                signals.append(None)            # hold
        return {"AAPL": signals}                # MSFT omitted = hold throughout

config = oxide.BacktestConfig(starting_cash=100_000, commission=0.001, slippage=0.0005)
result = oxide.run_backtest(strategy=MACrossover(), data=[aapl, msft], config=config)

print(result.sharpe, result.max_drawdown)
for trade in result.trades:
    print(trade.pnl)
```

### Analysing results

```python
pd.Series(result.summary())                                        # all scalar metrics
equity = pd.DataFrame(result.equity_curve, columns=["ts", "equity"])
equity["ts"] = pd.to_datetime(equity["ts"], unit="ms")
trades = pd.DataFrame([t.to_dict() for t in result.trades])
```

| `BacktestResult` | |
|---|---|
| Metrics | `total_return`, `annualized_return`, `annualized_volatility`, `sharpe`, `sortino`, `calmar`, `max_drawdown`, `max_drawdown_duration`, `win_rate`, `profit_factor`, `avg_trade_pnl`, `exposure`, `num_trades` |
| Series | `equity_curve`, `drawdown_curve`: `list[(timestamp_ms, value)]` |
| Trades | `trades`: `list[Trade]` with `symbol, side, quantity, entry/exit_timestamp, entry/exit_price, pnl, return_pct, commission, bars_held`, plus `to_dict()` |
| Other | `starting_cash`, `final_equity`, `config`, `summary()` |

### Signals

`Signal.long(w)`, `Signal.short(w)`, `Signal.flat()`, `Signal.scale_in(w)`, `Signal.scale_out(w)`.
`w` is a fraction of portfolio equity in `[0, 1]`. `None` means hold. Python only
expresses the target exposure. Rust resolves weight → dollars → units → order → fill.

### Strategy contract

```python
def generate_signals(self, data: dict[str, list[Bar]]) -> dict[str, list[Signal | None]]
```

- `signals[sym][i]` applies to `data[sym][i]`. Each list has exactly `len(data[sym])`
  entries; a symbol you omit holds on every bar.
- **Fill timing:** a signal on bar `i` of a symbol is sized at that bar's close and
  fills at the open of **that symbol's** bar `i + 1`, however much time passes in
  between (a gap, a holiday, a different trading calendar). A signal on a symbol's
  last bar never fills.
- Fill price is `open × (1 ± slippage)` (worse for you); commission is
  `|qty| × price × commission`.
- Signals from different symbols at the same timestamp are sized against the same
  equity snapshot. Positions still open at the end are closed at each symbol's
  final close, with the same costs.
- Rust merges all timestamps into one timeline internally to order events and to
  produce one `equity_curve` point per timestamp. Strategies never index into it.

### Multiple tickers and look-ahead

Tickers may have different dates. For a cross-ticker rule, align by timestamp and
only use data **known at or before** the bar you are signalling on:

```python
class LeadLag:  # trade MSFT, only reacting when an MSFT bar arrives
    def generate_signals(self, data):
        aapl = pd.Series({b.timestamp: b.close for b in data["AAPL"]})
        msft_ts = [b.timestamp for b in data["MSFT"]]
        known = aapl.reindex(aapl.index.union(msft_ts)).ffill().reindex(msft_ts)
        rising = (known.diff() > 0).tolist()
        return {"MSFT": [Signal.long(0.5) if up else Signal.flat() for up in rising]}
```

Before trusting a backtest (and before trading a strategy live), check it for
look-ahead bias:

```python
from oxide_engine.testing import assert_causal
assert_causal(LeadLag(), [aapl, msft])   # AssertionError names the symbol and bar index
```

It reruns `generate_signals` on truncated data and fails if any signal changes once
later bars exist (a wrong `shift`, `center=True`, backward fills, whole-history
normalization). Because signals are per bar, the same code runs live: on each new
bar, call `generate_signals` on the history so far and act on each symbol's last
entry.

### Data

`from_csv(path, symbol=None, schema=None)` produces canonical bars:
`timestamp` (Unix ms, UTC), `open`, `high`, `low`, `close`, `volume`.
Without a schema, headers are normalized (`<CLOSE>` → `close`). Rows with
inconsistent OHLC values or bad formatting raise `ValueError`. You can also build
data directly: `oxide.Ticker("AAPL", [oxide.Bar(ts, o, h, l, c, v), ...])`.

## Development

Requires Rust ≥ 1.83 and Python ≥ 3.9.

```bash
python -m venv .venv
source .venv/bin/activate            # Windows: .venv\Scripts\activate
pip install -e ".[dev]"              # or: pip install maturin pytest pytest-cov pandas
maturin develop                      # build the Rust extension into the venv

pytest                               # Python contract + spec tests
cargo test                           # Rust unit tests
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Re-run `maturin develop` after every Rust change.

## Project layout

```
src/
  lib.rs            #[pymodule] oxide_engine._core (exports only)
  error.rs          OxideError + the one From<OxideError> for PyErr
  data/             Bar, Ticker, ColumnSchema, CSV loader, timestamp parsing
  strategy/         Signal, SignalFrame
  execution/        sizing, Order, Fill, FillSimulator
  portfolio/        Portfolio, Position (signed qty), Trade
  metrics.rs        Sharpe, Sortino, Calmar, drawdown, ...
  backtest/         BacktestConfig, engine (event loop), BacktestResult
  bindings/         ALL PyO3 wrappers + run_backtest
python/oxide_engine/  __init__.py, data_loader.py, testing.py (assert_causal), _core.pyi, py.typed
tests/              pytest suite
examples/           runnable strategies
```

## Not modelled yet

Fractional quantities are allowed (no lot size). Slippage and commission are flat
percentages: no volume, spread, or partial-fill models, and no short-borrow
constraints.
