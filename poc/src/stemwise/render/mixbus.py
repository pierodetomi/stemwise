"""Mix bus: stereo summing and true-peak trim (REN-5), raw sum `baseline.wav` (REN-7).

Formulas: docs/analysis.md, section "Bus di mix". The bus is always stereo (D-008) and its only
processing is an attenuation-only trim below the true-peak ceiling (D-007).
"""

from __future__ import annotations

import math
from collections.abc import Iterable
from dataclasses import dataclass

import numpy as np

from stemwise.analysis.stem import true_peak_db
from stemwise.audio_io import Stems

# Constant-power centre for mono stems (same law as the pan of REN-3): -3.01 dB per channel.
MONO_CENTER_GAIN = math.sqrt(0.5)
# Target = ceiling - margin, so that 24-bit rounding of the written file cannot cross the ceiling.
TRIM_MARGIN_DB = 0.01


def to_stereo(audio: np.ndarray) -> np.ndarray:
    """Mono -> centred at constant power; stereo unchanged. Returns float32 (frames, 2)."""
    if audio.shape[1] == 2:
        return audio.astype(np.float32, copy=False)
    centred = audio[:, :1].astype(np.float64) * MONO_CENTER_GAIN
    return np.repeat(centred, 2, axis=1).astype(np.float32)


def sum_bus(stems: Iterable[np.ndarray]) -> np.ndarray:
    """Sum equal-length stems in the given order on a stereo bus (float64 accumulator)."""
    total: np.ndarray | None = None
    for audio in stems:
        stereo = to_stereo(audio).astype(np.float64)
        total = stereo if total is None else total + stereo
    if total is None:
        raise ValueError("the mix bus needs at least one stem")
    return total.astype(np.float32)


def trim_gain_db(true_peak: float, ceiling_db: float) -> float:
    """Attenuation that brings `true_peak` to `ceiling_db - TRIM_MARGIN_DB`; 0 if already below."""
    return min(0.0, (ceiling_db - TRIM_MARGIN_DB) - true_peak)


def trim(bus: np.ndarray, ceiling_db: float) -> tuple[np.ndarray, float]:
    """Return `(trimmed bus, gain in dB)`. Attenuation only: a quiet bus is returned unchanged."""
    gain_db = trim_gain_db(true_peak_db(bus), ceiling_db)
    if gain_db == 0.0:
        return bus, 0.0
    gain = 10.0 ** (gain_db / 20.0)
    return (bus.astype(np.float64) * gain).astype(np.float32), gain_db


@dataclass(frozen=True)
class BusResult:
    audio: np.ndarray
    true_peak_before_db: float
    trim_gain_db: float
    true_peak_db: float


def render_baseline(stems: Stems, ceiling_db: float) -> BusResult:
    """Raw sum of the original stems with the trim only (REN-7): no pre-gain, no plan."""
    bus = sum_bus(stems.audio.values())
    before = true_peak_db(bus)
    trimmed, gain_db = trim(bus, ceiling_db)
    after = true_peak_db(trimmed) if gain_db else before
    return BusResult(trimmed, before, gain_db, after)
