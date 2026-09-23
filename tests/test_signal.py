"""Signal: weight-only intent expressed by Python strategies."""

import math

import pytest

from oxide_engine import Signal

WEIGHTED_FACTORIES = [
    (Signal.long, "long"),
    (Signal.short, "short"),
    (Signal.scale_in, "scale_in"),
    (Signal.scale_out, "scale_out"),
]


@pytest.mark.parametrize(("factory", "kind"), WEIGHTED_FACTORIES)
def test_weighted_factories_record_kind_and_weight(factory, kind):
    signal = factory(0.25)

    assert signal.kind == kind
    assert signal.weight == 0.25


def test_flat_has_no_weight():
    signal = Signal.flat()

    assert signal.kind == "flat"
    assert signal.weight is None


@pytest.mark.parametrize("weight", [0.0, 1.0])
def test_weight_bounds_are_inclusive(weight):
    assert Signal.long(weight).weight == weight


@pytest.mark.parametrize(("factory", "_kind"), WEIGHTED_FACTORIES)
@pytest.mark.parametrize("weight", [-0.01, 1.01, math.nan, math.inf])
def test_out_of_range_weight_raises_value_error(factory, _kind, weight):
    with pytest.raises(ValueError, match="weight"):
        factory(weight)


def test_signals_compare_by_value():
    assert Signal.long(0.5) == Signal.long(0.5)
    assert Signal.long(0.5) != Signal.short(0.5)
    assert Signal.flat() == Signal.flat()


def test_repr_is_readable():
    assert repr(Signal.long(0.9)) == "Signal.long(0.9)"
    assert repr(Signal.flat()) == "Signal.flat()"
