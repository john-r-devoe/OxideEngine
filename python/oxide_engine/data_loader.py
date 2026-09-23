"""Load market data from arbitrary CSV layouts into :class:`oxide_engine.Ticker`."""

from __future__ import annotations

import os
from typing import Mapping, Optional, Union

from oxide_engine import _core

__all__ = ["from_csv"]


def from_csv(
    path: Union[str, "os.PathLike[str]"],
    symbol: Optional[str] = None,
    schema: Optional[Mapping[str, str]] = None,
) -> _core.Ticker:
    """Load one symbol's OHLCV bars from a CSV file.

    Args:
        path: CSV file path.
        symbol: Ticker symbol. Defaults to the file name up to the first dot,
            upper-cased (``"AAPL.us.txt"`` -> ``"AAPL"``).
        schema: Maps your column names to canonical fields
            (``timestamp``, ``open``, ``high``, ``low``, ``close``, ``volume``).
            When omitted, headers are normalized (``<CLOSE>`` -> ``close``) and
            matched automatically.

    Raises:
        ValueError: Invalid schema, missing columns, malformed or OHLC-inconsistent rows.
        OSError: The file cannot be read.
    """
    return _core.data_loader.from_csv(os.fspath(path), symbol, dict(schema) if schema is not None else None)
