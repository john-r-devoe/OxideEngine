"""BacktestConfig construction and validation."""

import math

import pytest

import oxide_engine as oxide


def test_accepts_int_starting_cash_and_exposes_fields():
    config = oxide.BacktestConfig(starting_cash=100_000, commission=0.001, slippage=0.0005)

    assert config.starting_cash == 100_000.0
    assert config.commission == 0.001
    assert config.slippage == 0.0005


def test_optional_fields_have_defaults():
    config = oxide.BacktestConfig(starting_cash=50_000)

    assert config.commission == 0.0
    assert config.slippage == 0.0
    assert config.risk_free_rate == 0.0
    assert config.periods_per_year == 252.0


@pytest.mark.parametrize("cash", [0, -1, math.nan, math.inf])
def test_rejects_non_positive_or_non_finite_cash(cash):
    with pytest.raises(ValueError, match="starting_cash"):
        oxide.BacktestConfig(starting_cash=cash)


@pytest.mark.parametrize("field", ["commission", "slippage"])
@pytest.mark.parametrize("value", [-0.001, 1.0, math.nan])
def test_rejects_cost_fractions_outside_unit_interval(field, value):
    with pytest.raises(ValueError, match=field):
        oxide.BacktestConfig(starting_cash=1_000, **{field: value})


def test_rejects_non_positive_periods_per_year():
    with pytest.raises(ValueError, match="periods_per_year"):
        oxide.BacktestConfig(starting_cash=1_000, periods_per_year=0)
