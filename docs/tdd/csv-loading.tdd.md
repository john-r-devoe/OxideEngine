# TDD evidence: CSV loading (`data::csv_loader`, `data::schema`, `data::timestamp`)

**Source plan:** none. Journeys were derived from the stub doc comments and the user's request:
"map data to a default format if no schema mapping is provided and error if impossible. If a
schema is provided, attempt to map it, and error if not all required fields are present."

## User journeys

1. As a strategy author, I load a file with recognizable headers (`timestamp`/`date`, `open`, ...,
   or Stooq's `<DATE>,<OPEN>,...`) with no schema, and the symbol is inferred from the file name.
   Intraday data carries its time in the same cell (`2024-01-02 09:30:00`).
2. As a strategy author, I load a CSV with my own headers by passing
   `{"<my column>": "<canonical field>"}`.
3. As a strategy author, I get a clear `ValueError` naming the problem (missing or ambiguous
   column, bad row number, OHLC violation, duplicate timestamp) instead of silent bad data.

## Design

- `schema::locate` is the single place that maps headers to a `ColumnIndex`. It errors if any
  canonical field matches zero columns or more than one.
- `ColumnSchema::resolve` matches by the user's mapping. `auto_detect` matches normalized
  headers (strip `<>`, trim, lowercase) against a small alias table. Other columns (e.g. Stooq's
  `<TIME>`) are ignored: date and time must be in one timestamp column.
- `csv_loader::load_csv` does the following, in order:
  1. reads the header;
  2. resolves columns (with the schema, otherwise auto-detect);
  3. parses each row into `Bar::new`, reporting failures as `MalformedRow { row }` (1-based);
  4. sorts by timestamp and rejects duplicates;
  5. uses the explicit symbol, otherwise `infer_symbol`.
- `timestamp::parse_timestamp` has no dependencies. It accepts:
  - `YYYYMMDD`;
  - `YYYY-MM-DD` with an optional ` HH:MM:SS` or `THH:MM:SS`;
  - integer epoch seconds or milliseconds (values ≥ 1e11 are treated as milliseconds).

  Its implementation was also a stub, and loading needs it.

## Checkpoints

| Commit | Stage | Evidence |
|--------|-------|----------|
| `242be07` | RED | `cargo test --lib data::`: 13 passed, 20 failed (all on `NotImplemented`/`todo!`). `pytest tests/test_data_loader.py`: 8 failed (`NotImplementedError`), 2 passed |
| `f31932c` | GREEN | `cargo test --lib`: 51 passed. `pytest`: 74 passed, 1 xfailed. `clippy -D warnings`: clean |
| `6e4469a` | RED (review) | 32 passed, 2 failed: an overflow panic at `timestamp.rs:99` for `99999999999999999-01-01`, and a `Vol`+`Volume` header was accepted |
| `c4b9c0e` | GREEN (review) | `cargo test --lib`: 52 passed. `pytest`: 74 passed, 1 xfailed. clippy clean |
| `68f9fa1` | RED (simplify) | Compile-time RED: E0063 missing `ColumnIndex.time`, E0061 `parse_timestamp` still takes 2 args. `pytest`: 1 failed (`Z` suffix accepted), 11 passed |
| `f10d548` | GREEN (simplify) | `cargo test --lib`: 53 passed. `pytest`: 76 passed, 1 xfailed. clippy clean |

## Test specification

| # | Guarantee | Test | Type | Result |
|---|-----------|------|------|--------|
| 1 | Stooq headers auto-detect; a `<TIME>` column is ignored | `schema::tests::auto_detects_stooq_headers_ignoring_time_column` | unit | PASS |
| 2 | Plain headers auto-detect in any order, case-insensitively | `schema::tests::auto_detects_plain_headers_in_any_order` | unit | PASS |
| 3 | Auto-detect names every missing field | `schema::tests::auto_detect_names_every_missing_field` | unit | PASS |
| 4 | Auto-detect rejects two columns matching one field | `schema::tests::auto_detect_rejects_two_columns_matching_one_field` | unit | PASS |
| 5 | A user schema resolves against the header | `schema::tests::resolves_user_schema_against_header` | unit | PASS |
| 6 | A schema column absent from the file, or a schema missing a field, is an error | `schema::tests::resolve_errors_*` | unit | PASS |
| 7 | All documented timestamp formats parse; invalid, out-of-range and overflow inputs are errors, never panics | `timestamp::tests::*` | unit | PASS |
| 8 | Symbol is inferred from the file name, and a stem-less name is rejected | `csv_loader::tests::infers_symbol_from_file_name`, `rejects_file_name_without_a_stem` | unit | PASS |
| 9 | Rows are sorted; duplicate timestamps are rejected | `csv_loader::tests::sorts_rows_by_timestamp`, `rejects_duplicate_timestamps` | unit | PASS |
| 10 | Bad cell or OHLC violation → `MalformedRow` with a 1-based row number | `csv_loader::tests::reports_*` | unit | PASS |
| 11 | Missing file → `OSError`; unrecognized headers → `ValueError` suggesting a schema | `tests/test_data_loader.py` | integration | PASS |
| 12 | Stooq, custom-schema and intraday `YYYY-MM-DD HH:MM:SS` files load through the Python API | `tests/test_data_loader.py` | integration | PASS |
| 13 | Timezone designators (`Z`, `+02:00`, `-05:00`) are rejected | `timestamp::tests::rejects_timezone_designators`, `test_timezone_designator_raises_value_error` | unit + integration | PASS |

## Coverage and known gaps

`cargo llvm-cov --lib --summary-only` gives this line coverage:

| File | Line coverage |
|------|---------------|
| `schema.rs` | 100% |
| `csv_loader.rs` | 99% |
| `timestamp.rs` | 98% |

The crate total is 65%, because the engine, simulator and metrics are still stubs.

Out of scope by design (data is expected to be cleaned before loading):
- Separate date and time columns. Combine them into one timestamp column.
- Timezones. Timestamps are naive and read as UTC. Every file in a backtest must use the same
  clock, because signal row `i` aligns bars across tickers by exact timestamp.

Known gaps:
- `tests/test_end_to_end.py` stays strict-xfail until `engine::run` is implemented.
