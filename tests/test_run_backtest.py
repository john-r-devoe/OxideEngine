"""run_backtest: the single-FFI-crossing contract.

Rust must call `strategy.generate_signals(data)` exactly once with every bar
for every asset, then run the whole simulation without calling back into Python.
"""

import pytest

import oxide_engine as oxide
from oxide_engine import Signal


class SpyStrategy:
    """Records every call and returns a valid all-hold signal list."""

    def __init__(self):
        self.calls = []

    def generate_signals(self, data):
        self.calls.append(data)
        n = max(len(bars) for bars in data.values())
        return [{symbol: None for symbol in data} for _ in range(n)]


class ReturnsFn:
    def __init__(self, fn):
        self._fn = fn

    def generate_signals(self, data):
        return self._fn(data)


def test_generate_signals_is_called_exactly_once_with_all_assets(aapl, msft, config):
    strategy = SpyStrategy()

    with pytest.raises(NotImplementedError):  # engine loop is stubbed
        oxide.run_backtest(strategy=strategy, data=[aapl, msft], config=config)

    assert len(strategy.calls) == 1
    data = strategy.calls[0]
    assert set(data) == {"AAPL", "MSFT"}
    assert len(data["AAPL"]) == len(aapl)
    assert all(isinstance(bar, oxide.Bar) for bar in data["AAPL"])
    assert data["MSFT"][0].open == msft[0].open


def test_strategy_without_generate_signals_raises_attribute_error(aapl, config):
    with pytest.raises(AttributeError, match="generate_signals"):
        oxide.run_backtest(strategy=object(), data=[aapl], config=config)


def test_non_list_return_raises_type_error(aapl, config):
    with pytest.raises(TypeError, match="list"):
        oxide.run_backtest(strategy=ReturnsFn(lambda d: "nope"), data=[aapl], config=config)


def test_non_dict_row_raises_type_error_with_index(aapl, config):
    strategy = ReturnsFn(lambda d: [{"AAPL": None}, ["not", "a", "dict"]])

    with pytest.raises(TypeError, match="index 1"):
        oxide.run_backtest(strategy=strategy, data=[aapl], config=config)


def test_non_signal_value_raises_type_error_with_index_and_symbol(aapl, config):
    strategy = ReturnsFn(lambda d: [{"AAPL": Signal.flat()}, {"AAPL": 0.5}])

    with pytest.raises(TypeError, match=r"index 1.*AAPL"):
        oxide.run_backtest(strategy=strategy, data=[aapl], config=config)


def test_unknown_symbol_in_signals_raises_value_error(aapl, config):
    strategy = ReturnsFn(lambda d: [{"TSLA": Signal.long(0.5)}])

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
