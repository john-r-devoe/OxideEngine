"""The import surface promised by the README usage example."""

import oxide_engine as oxide


def test_top_level_names_are_exported():
    for name in ("Bar", "Ticker", "Signal", "BacktestConfig", "BacktestResult", "Trade", "run_backtest", "data_loader"):
        assert hasattr(oxide, name), f"oxide_engine.{name} missing"


def test_signal_importable_from_package():
    from oxide_engine import Signal

    assert Signal is oxide.Signal


def test_data_loader_is_a_submodule_with_from_csv():
    from oxide_engine.data_loader import from_csv

    assert callable(from_csv)
    assert oxide.data_loader.from_csv is from_csv


def test_version_is_exposed():
    assert isinstance(oxide.__version__, str)
    assert oxide.__version__.count(".") == 2
