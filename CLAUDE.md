# CLAUDE.md: Oxide Engine

Guidance for AI agents and contributors. Read this before changing code.

## What this is

A backtesting engine: Python strategies, Rust execution, glued with PyO3 and
built with maturin (mixed layout). The Python package is `oxide_engine` and the
compiled extension is `oxide_engine._core`.

## THE architectural constraint (never violate)

**One FFI crossing per backtest.** `bindings::backtest::run_backtest`:

1. validates the universe (`engine::validate_universe`);
2. builds `{symbol: [Bar, ...]}` for all assets;
3. calls `strategy.generate_signals(data)` **exactly once**;
4. converts the returned `list[dict[str, Signal | None]]` into a `SignalFrame`;
5. releases the GIL (`py.detach`) and runs `backtest::engine::run` in pure Rust.

Never add per-bar Python callbacks, "on_bar" hooks, or Python calls inside
`engine`, `execution`, `portfolio`, or `metrics`. If a feature seems to need one,
make the strategy return more information up front instead.
`tests/test_run_backtest.py::test_generate_signals_is_called_exactly_once_with_all_assets`
guards this constraint.

## Domain decisions (already made; do not re-litigate)

- **Bar**: `timestamp: i64` (Unix ms UTC), `open/high/low/close/volume: f64`.
  `Bar::new` enforces finite prices > 0, `low ≤ open,close ≤ high`, volume ≥ 0.
- **Position** uses a signed `quantity` (+ long, − short). There is no separate
  short or debt struct. Invariant: `equity = cash + Σ quantity × price`.
- **Signal**: `Long(w) | Short(w) | Flat | ScaleIn(w) | ScaleOut(w)`, with
  `w ∈ [0, 1]` as a fraction of equity. Python `None` means hold, stored as a
  missing key in `SignalRow`. Python sends weights only. Rust owns weight → target
  $ → delta units → order → fill (`execution::sizing`).
- **Timeline**: the sorted union of all tickers' timestamps. Signal row `i`
  applies at `timeline[i]` (`engine::build_timeline`).
- **Fill timing**: a signal on bar `t` fills at the **open of bar `t+1`**,
  price `open × (1 ± slippage)`, commission `|qty| × price × commission`.
- **Trades** are round trips. Open positions are closed at the last bar's close.
- **Metrics**: annualized with `config.periods_per_year` (default 252).
  Drawdowns are positive fractions.
- **Errors**: domain code returns `OxideResult<T>`. `From<OxideError> for PyErr`
  in `src/error.rs` maps `Io` to `OSError`, `NotImplemented` to
  `NotImplementedError`, and everything else to `ValueError`. Code reachable from
  Python must never panic. `todo!()` is only allowed in functions that are not
  yet reachable from Python.

## Layout

```
src/lib.rs                 module tree + #[pymodule] mod _core (exports only)
src/error.rs               OxideError, OxideResult, PyErr conversion
src/data/                  bar, ticker, schema (CanonicalField, ColumnSchema), csv_loader, timestamp
src/strategy/              signal, signal_frame
src/execution/             order, fill, sizing, simulator
src/portfolio.rs (+ /)     Portfolio, position, trade
src/metrics.rs             statistics
src/backtest/              config, engine, result
src/bindings/              data.rs (Bar, Ticker, from_csv), signal.rs, backtest.rs (config, Trade, result, run_backtest)
python/oxide_engine/       __init__.py (re-exports), data_loader.py (PathLike wrapper), _core.pyi (stubs), py.typed
tests/                     pytest; conftest.py holds fixtures and synthetic CSV builders
examples/ma_crossover.py   the README example
docs/tdd/                  TDD evidence reports
```

Module rules (full detail in `.claude/rules/rust/pyo3.md`):
- `#[pyclass]` / `#[pyfunction]` / `#[pymethods]` appear **only** in `src/bindings/`.
  Domain modules import no PyO3 (the one exception is `error.rs`'s `From` impl).
- Bindings are thin `Py*` wrappers (`inner: DomainType`) with `name = "..."`,
  `module = "oxide_engine"`, `frozen`. Validation belongs in domain constructors.
- A new Python-visible item needs its `#[pymodule_export]` in `lib.rs`, its
  signature in `_core.pyi`, and a re-export in `__init__.py`, all in the same commit.
- Uses the `mod.rs`-less layout (`foo.rs` + `foo/`).

## Workflow

```bash
# one-time
python -m venv .venv && .venv/Scripts/activate   # (POSIX: source .venv/bin/activate)
pip install -e ".[dev]"

# every change
maturin develop                                   # REQUIRED after any Rust change
pytest                                            # Python suite
cargo test                                        # Rust unit tests (PYO3_PYTHON may need to point at .venv python)
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo llvm-cov --summary-only                     # Rust coverage (optional)
pytest --cov=oxide_engine                         # Python coverage
```

- `extension-module` is an **opt-in** cargo feature that maturin enables
  (`pyproject.toml`). Leave it off for `cargo test` so the test binary links libpython.
- TDD is mandatory (`.claude/rules/common/testing.md`): write or adjust the failing
  test first, confirm RED, implement, confirm GREEN, refactor.
  Use conventional-commit checkpoints (`test:` then `feat:`/`fix:` then `refactor:`).
- Unit tests go in `#[cfg(test)] mod tests` inside each Rust file. Behavior visible
  from Python gets a pytest test.

### Implementing a stub

Stubs return `OxideError::NotImplemented("<what> (<module path>)")` or call `todo!()`.
Their expected behavior is written in the doc comment and in a strict-xfail pytest
(`xfail_strict = true` in `pyproject.toml`):

1. Pick the stub. Read its doc comment and the matching xfail test in `tests/`.
2. Add Rust unit tests for the new logic (RED).
3. Implement it, then `cargo test`, `maturin develop`, `pytest`.
4. The xfail test now XPASSes, which fails the run. Delete its `xfail` marker (GREEN).

Suggested order: csv_loader/timestamp/schema → build_timeline → sizing →
simulator → Portfolio::apply_fill → metrics → engine::run.
`tests/test_end_to_end.py` (the README example) is the final acceptance test.

## Gotchas

- Python 3.14 is in the local venv. pyo3 0.29 with `abi3-py39` builds one wheel
  for Python ≥ 3.9.
- PyO3 0.29 APIs: use `Bound::cast` (not `downcast`), `py.detach` (not
  `allow_threads`), and `skip_from_py_object` on `Clone` pyclasses.
- `Ticker` stores bars in `Arc<[Bar]>`, so clones are cheap. Don't change it to `Vec`.
- Returning `Vec<PyBar>` copies every bar. For large series, prefer
  `__getitem__`/`__len__`.
