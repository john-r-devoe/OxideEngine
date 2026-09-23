"""data_loader.from_csv: source-agnostic CSV ingestion.

Tests marked `xfail(strict=True)` are the executable spec for the stubbed CSV
parser. When the parser is implemented they XPASS, which fails the run — delete
the marker at that point.
"""

from pathlib import Path

import pytest

import oxide_engine as oxide
from conftest import CUSTOM_SCHEMA

pending_parser = pytest.mark.xfail(raises=NotImplementedError, strict=True, reason="CSV parser is stubbed")


def test_schema_with_unknown_canonical_name_raises_value_error(custom_csv):
    with pytest.raises(ValueError, match="adj_close"):
        oxide.data_loader.from_csv(str(custom_csv), schema={"Close": "adj_close"})


def test_schema_mapping_two_columns_to_one_field_raises_value_error(custom_csv):
    with pytest.raises(ValueError, match="close"):
        oxide.data_loader.from_csv(str(custom_csv), schema={"Close": "close", "Adj Close": "close"})


def test_accepts_pathlike_and_reaches_stub_parser(custom_csv: Path):
    with pytest.raises(NotImplementedError):
        oxide.data_loader.from_csv(custom_csv, schema=CUSTOM_SCHEMA)


@pending_parser
def test_auto_normalizes_stooq_headers_and_infers_symbol(stooq_csv: Path):
    ticker = oxide.data_loader.from_csv(str(stooq_csv))

    assert ticker.symbol == "AAPL"
    assert len(ticker) == 60
    assert ticker[0].open == 100.0


@pending_parser
def test_loads_custom_headers_through_schema(custom_csv: Path):
    ticker = oxide.data_loader.from_csv(str(custom_csv), symbol="MSFT", schema=CUSTOM_SCHEMA)

    assert ticker.symbol == "MSFT"
    assert len(ticker) == 2
    assert ticker[0].timestamp == 1_704_153_600_000
    assert ticker[1].volume == 1200.0


@pending_parser
def test_malformed_row_raises_value_error_with_row_number(tmp_path: Path):
    path = tmp_path / "bad.csv"
    path.write_text("Date,Open,High,Low,Close,Vol\n2024-01-02,10,11,9,10.5,100\n2024-01-03,abc,11,9,10,100\n")

    with pytest.raises(ValueError, match="row 2"):
        oxide.data_loader.from_csv(str(path), symbol="X", schema=CUSTOM_SCHEMA)


@pending_parser
def test_ohlc_inconsistent_row_raises_value_error(tmp_path: Path):
    path = tmp_path / "bad.csv"
    path.write_text("Date,Open,High,Low,Close,Vol\n2024-01-02,10,9,11,10.5,100\n")

    with pytest.raises(ValueError, match="high"):
        oxide.data_loader.from_csv(str(path), symbol="X", schema=CUSTOM_SCHEMA)


@pending_parser
def test_missing_required_column_raises_value_error(tmp_path: Path):
    path = tmp_path / "bad.csv"
    path.write_text("Date,Open,High,Low,Vol\n2024-01-02,10,11,9,100\n")

    with pytest.raises(ValueError, match="close"):
        oxide.data_loader.from_csv(str(path), symbol="X", schema=CUSTOM_SCHEMA)
