"""Loudness measures: integrated loudness (ANA-1) and loudness range (ANA-2, D-015).

Both come from `pyloudnorm` (ITU-R BS.1770-4 K-weighting and gating; EBU Tech 3342 loudness
range). A new meter is created for every call, because the library keeps per-call state on it.
Formulas and parameters: docs/analysis.md, "Loudness".
"""

from __future__ import annotations

import math

import numpy as np
import pyloudnorm

INTEGRATED_BLOCK_S = 0.4  # BS.1770 gating block


def _as_float64(audio: np.ndarray) -> np.ndarray:
    a = np.asarray(audio, dtype=np.float64)
    return a[:, None] if a.ndim == 1 else a


def integrated_lufs(audio: np.ndarray, sample_rate: int) -> float | None:
    """Integrated loudness in LUFS (BS.1770-4, gates -70 LUFS and -10 LU).

    -inf when every block is gated out (e.g. digital silence); None when the signal is shorter
    than one 400 ms gating block.
    """
    a = _as_float64(audio)
    if a.shape[0] < INTEGRATED_BLOCK_S * sample_rate:
        return None
    value = float(pyloudnorm.Meter(sample_rate).integrated_loudness(a))
    return value if not math.isnan(value) else None


def loudness_range_lu(audio: np.ndarray, sample_rate: int) -> float | None:
    """Loudness range in LU (EBU Tech 3342): spread between the 10th and 95th percentile of the
    short-term (3 s) loudness, gated at -70 LUFS absolute and -20 LU relative.

    None when the signal is too short for one short-term window or entirely gated out.
    """
    a = _as_float64(audio)
    try:
        value = float(pyloudnorm.Meter(sample_rate).loudness_range(a))
    except ValueError:  # shorter than a 3 s window, even after the 1.5 s of appended silence
        return None
    return value if math.isfinite(value) else None


def gain_to_lufs(current_lufs: float, target_lufs: float) -> float:
    """Linear gain that moves a signal measured at `current_lufs` to `target_lufs`."""
    return 10.0 ** ((target_lufs - current_lufs) / 20.0)
