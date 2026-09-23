# TDD evidence: Oxide Engine skeleton

- **Branch:** `complete-rescope`
- **Source plan:** none. The user journeys below were derived from the scaffold request during this run.
- **Environment:** Windows 11, Rust 1.91.1, Python 3.14.0 (`.venv`), maturin 1.15.0, pyo3 0.29.2, pytest 9.1.1, pandas 3.0.6

## User journeys

1. As a quant, I want to load any OHLCV CSV with my own header names, so that I'm not locked to one data vendor.
2. As a quant, I want to write a strategy as one vectorized `generate_signals(data)` call, so that the backtest never pays per-bar FFI overhead.
3. As a quant, I want to express intent as equity weights (`Signal.long(0.9)`), so that sizing, orders and fills are handled for me.
4. As a quant, I want metrics, equity/drawdown curves and every trade back as Python objects, so that I can plot and analyse them with pandas.

## RED → GREEN

| Stage | Commit | Command | Result |
|---|---|---|---|
| RED | `6b851a5` | `maturin develop` (empty `_core`), then `pytest --continue-on-collection-errors` | **35 failed, 6 errors.** All are `AttributeError`/`ImportError` for the missing API (`Bar`, `Ticker`, `Signal`, `BacktestConfig`, `run_backtest`, `data_loader`) |
| GREEN | `81488c5` | `cargo clippy --all-targets -- -D warnings` | clean |
| GREEN | `81488c5` | `cargo test` | **31 passed** |
| GREEN | `81488c5` | `maturin develop && pytest` | **67 passed, 6 xfailed** (strict) |
| Refactor | *(refactor commit)* | `cargo fmt`, fixes from the code review, then the full suite again | see "Refactor" below |

Rust unit tests were written in the same step as the domain modules (colocated `#[cfg(test)]`), so they have no separate RED run. The Python contract suite was the RED gate.

## Test specification

| # | What is guaranteed | Test | Type | Result |
|---|---|---|---|---|
| 1 | `import oxide_engine as oxide`, `from oxide_engine import Signal` and `from oxide_engine.data_loader import from_csv` all work, and `__version__` is exposed | `tests/test_public_api.py` | integration | PASS |
| 2 | Signal factories keep kind and weight. Weight must be in [0, 1] and finite, otherwise `ValueError`. Signals compare by value. `repr` is readable | `tests/test_signal.py`, `src/strategy/signal.rs` | unit + integration | PASS |
| 3 | `BacktestConfig` accepts int cash and has defaults. It rejects bad `starting_cash`, `commission`/`slippage` outside [0, 1), and `periods_per_year` ≤ 0 | `tests/test_config.py`, `src/backtest/config.rs` | unit + integration | PASS |
| 4 | `Bar` rejects inconsistent OHLCV (high < low/close, low > open, non-positive or NaN price, negative volume) | `tests/test_bar_ticker.py`, `src/data/bar.rs` | unit + integration | PASS |
| 5 | `Ticker` supports `len`, negative indexing and `IndexError`, and rejects a blank symbol or empty bars | `tests/test_bar_ticker.py`, `src/data/ticker.rs` | unit + integration | PASS |
| 6 | A schema with an unknown canonical name, or two columns mapped to one field, raises `ValueError` before any file I/O | `tests/test_data_loader.py`, `src/data/schema.rs` | unit + integration | PASS |
| 7 | **`generate_signals` is called exactly once**, with `{symbol: [Bar]}` for all assets | `test_run_backtest.py::test_generate_signals_is_called_exactly_once_with_all_assets` | integration | PASS |
| 8 | Bad strategy output (non-list, non-dict row, non-Signal value, unknown symbol) raises `TypeError`/`ValueError` naming the index and symbol. Strategy exceptions propagate. Empty or duplicate data raises `ValueError` | `tests/test_run_backtest.py`, `src/strategy/signal_frame.rs`, `src/backtest/engine.rs` | unit + integration | PASS |
| 9 | Equity = cash + Σ signed qty × price. Shorts have negative market value | `src/portfolio.rs`, `src/portfolio/position.rs` | unit | PASS |
| 10 | CSV auto-normalization, schema loading, malformed-row and OHLC errors, missing column | `tests/test_data_loader.py` (5 tests) | spec | XFAIL (strict; parser stubbed) |
| 11 | README example runs end-to-end and returns metrics, curves and trades | `tests/test_end_to_end.py` | E2E spec | XFAIL (strict; engine stubbed) |

## Coverage and known gaps

- **Python:** `pytest --cov=oxide_engine` gives **100%** (10/10 statements).
- **Rust:** `cargo llvm-cov --summary-only` gives **44.3% lines** overall. Implemented domain logic is covered at 79–100% (`bar` 100%, `signal` 100%, `position` 100%, `config` 98%, `schema` 88%, `ticker` 84%, `signal_frame` 82%, `portfolio` 79%, `engine` 78%). The 0% files are either:
  - intentional stubs (`csv_loader`, `timestamp`, `sizing`, `simulator`, `metrics`, `result`); or
  - `src/bindings/*`, which pytest exercises but `cargo llvm-cov` can't see.
- The 80% target applies once the stubbed business logic is implemented. Each stub has a strict-xfail spec.

## Known design decisions to confirm

- Multi-asset alignment: signal row `i` applies at `timeline[i]`, where the timeline is the sorted union of all tickers' timestamps. The README example indexes by AAPL bars, which works when all assets share dates.
- Fills happen at the next bar's open, and open positions are force-closed at the last close.

## Refactor

Review: `ecc:rust-reviewer` approved with **no CRITICAL or HIGH findings**. It confirmed no panics on paths reachable from Python and that the single-FFI-crossing constraint holds.

| Finding | Action |
|---|---|
| MEDIUM: `_core.pyi` typed `data_loader` as a bare `ModuleType` | Fixed: `_DataLoaderModule` stub with the typed `from_csv` |
| MEDIUM: `fast[i-1]` KeyError at `i=0` in the README example | Not a bug: `fast[0]`/`slow[0]` are NaN, so `NaN > NaN` short-circuits the `and` before `fast[-1]` is evaluated. The example is also the user-specified verbatim usage. No change |
| LOW: wildcard arm in `From<OxideError> for PyErr` | Fixed: all variants listed explicitly |
| LOW: flat top-level exports | Intentional (the public API requires `oxide_engine.Bar` etc.). No change |

Also applied `cargo fmt` across the crate (CI enforces `cargo fmt --check`).

After the refactor: `cargo fmt --check` clean · `cargo clippy --all-targets -- -D warnings` clean · `cargo test` **31 passed** · `pytest --cov=oxide_engine` **67 passed, 6 xfailed, 100%** Python coverage.
