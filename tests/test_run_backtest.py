"""run_backtest: the single-FFI-crossing contract.

Rust must call `strategy.generate_signals(data)` exactly once with every bar
for every asset, then run the whole simulation without calling back into Python.
"""

import pytest

import oxide_engine as oxide
from oxide_engine import Signal


class SpyStrategy:
    """Records every call and returns a valid all-hold signal dict."""

    def __init__(self):
        self.calls = []

    def generate_signals(self, data):
        self.calls.append(data)
        return {symbol: [None] * len(bars) for symbol, bars in data.items()}


class ReturnsFn:
    def __init__(self, fn):
        self._fn = fn

    def generate_signals(self, data):
        return self._fn(data)


def test_generate_signals_is_called_exactly_once_with_all_assets(aapl, msft, config):
    strategy = SpyStrategy()

    result = oxide.run_backtest(strategy=strategy, data=[aapl, msft], config=config)

    assert isinstance(result, oxide.BacktestResult)

    assert len(strategy.calls) == 1
    data = strategy.calls[0]
    assert set(data) == {"AAPL", "MSFT"}
    assert len(data["AAPL"]) == len(aapl)
    assert all(isinstance(bar, oxide.Bar) for bar in data["AAPL"])
    assert data["MSFT"][0].open == msft[0].open


def test_strategy_without_generate_signals_raises_attribute_error(aapl, config):
    with pytest.raises(AttributeError, match="generate_signals"):
        oxide.run_backtest(strategy=object(), data=[aapl], config=config)


def test_non_dict_return_raises_type_error(aapl, config):
    with pytest.raises(TypeError, match="dict"):
        oxide.run_backtest(strategy=ReturnsFn(lambda d: [None] * len(aapl)), data=[aapl], config=config)


def test_non_str_symbol_key_raises_type_error(aapl, config):
    with pytest.raises(TypeError, match="non-str"):
        oxide.run_backtest(strategy=ReturnsFn(lambda d: {1: [None]}), data=[aapl], config=config)


def test_non_list_signals_for_symbol_raise_type_error(aapl, config):
    with pytest.raises(TypeError, match=r"'AAPL'.*list"):
        oxide.run_backtest(strategy=ReturnsFn(lambda d: {"AAPL": Signal.flat()}), data=[aapl], config=config)


def test_non_signal_element_raises_type_error_with_symbol_and_index(aapl, config):
    signals = [None] * len(aapl)
    signals[1] = 0.5
    strategy = ReturnsFn(lambda d: {"AAPL": signals})

    with pytest.raises(TypeError, match=r"'AAPL' at index 1"):
        oxide.run_backtest(strategy=strategy, data=[aapl], config=config)


def test_signal_count_must_match_symbol_bar_count(aapl, config):
    strategy = ReturnsFn(lambda d: {"AAPL": [None] * (len(aapl) - 1)})

    with pytest.raises(ValueError, match=r"'AAPL'.*entries.*bars"):
        oxide.run_backtest(strategy=strategy, data=[aapl], config=config)


def test_unknown_symbol_in_signals_raises_value_error(aapl, config):
    strategy = ReturnsFn(lambda d: {"TSLA": [Signal.long(0.5)]})

    with pytest.raises(ValueError, match="TSLA"):
        oxide.run_backtest(strategy=strategy, data=[aapl], config=config)


def test_exception_inside_strategy_propagates_unchanged(aapl, config):
    def boom(_data):
        raise RuntimeError("strategy exploded")

    with pytest.raises(RuntimeError, match="strategy exploded"):
        oxide.run_backtest(strategy=ReturnsFn(boom), data=[aapl], config=config)


def test_empty_data_raises_value_error(config):
    with pytest.raises(ValueError, match="data"):
        oxide.run_backtest(strategy=SpyStrategy(), data=[], config=config)


def test_duplicate_symbols_raise_value_error(aapl, config):
    with pytest.raises(ValueError, match="AAPL"):
        oxide.run_backtest(strategy=SpyStrategy(), data=[aapl, aapl], config=config)
