# Oxide Engine

A backtesting engine where **strategies are written in Python** and **execution runs in Rust**.

Your strategy is called **once** with every bar for every asset and returns every
signal for every timestep. Rust then turns those weights into orders, fills,
portfolio state, and an equity curve, without calling back into Python.

> **Status: skeleton.** The public API, domain model, and PyO3 bindings are in
> place and tested. Input validation works. CSV parsing, fill simulation,
> portfolio accounting, metrics, and the event loop are stubbed and raise
> `NotImplementedError`. See [Roadmap](#roadmap).

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
    def generate_signals(self, data: dict) -> list[dict]:
        closes = pd.Series([b.close for b in data["AAPL"]])
        fast = closes.rolling(20).mean()
        slow = closes.rolling(50).mean()
        signals = []
        for i in range(len(closes)):
            if fast[i] > slow[i] and fast[i-1] <= slow[i-1]:
                signals.append({"AAPL": Signal.long(0.9)})
            elif fast[i] < slow[i] and fast[i-1] >= slow[i-1]:
                signals.append({"AAPL": Signal.flat()})
            else:
                signals.append({"AAPL": None})
        return signals

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
A signal on bar `t` fills at the open of bar `t+1`.

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

Re-run `maturin develop` after every Rust change. See [CLAUDE.md](CLAUDE.md) for
the architecture, the rules, and the TDD workflow.

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
python/oxide_engine/  __init__.py, data_loader.py, _core.pyi, py.typed
tests/              pytest suite (strict-xfail tests = spec for stubbed features)
examples/           runnable strategies
```

## Roadmap

1. `data::csv_loader` + `data::timestamp` + `schema::{normalize_header, auto_detect, resolve}`
2. `backtest::engine::build_timeline`
3. `execution::sizing`, `execution::simulator`
4. `portfolio::Portfolio::apply_fill` (trade records)
5. `metrics::*`
6. `backtest::engine::run`

Every step has a strict-xfail test in `tests/`. When a feature starts passing,
its xfail test XPASSes and fails the run until you remove the marker.
