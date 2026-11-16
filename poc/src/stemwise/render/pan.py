"""Pan (REN-3): constant power for mono stems, balance for stereo stems (D-008, D-030).

Formulas: docs/analysis.md, "Render". The output is always stereo, float32 (frames, 2).
"""

from __future__ import annotations

import math

import numpy as np

from stemwise.render.mixbus import MONO_CENTER_GAIN, to_stereo


def mono_pan_gains(pan: float) -> tuple[float, float]:
    """`(L, R)` with `θ = (p + 1)·π/4`: `L = cos θ`, `R = sin θ`. Exact at -1, 0 and +1."""
    if pan <= -1.0:
        return 1.0, 0.0
    if pan >= 1.0:
        return 0.0, 1.0
    if pan == 0.0:
        return MONO_CENTER_GAIN, MONO_CENTER_GAIN
    theta = (pan + 1.0) * math.pi / 4.0
    return math.cos(theta), math.sin(theta)


def balance_gains(pan: float) -> tuple[float, float]:
    """Linear balance for stereo stems (D-030): only the opposite channel is attenuated."""
    return min(1.0, 1.0 - pan), min(1.0, 1.0 + pan)


def pan(audio: np.ndarray, position: float) -> np.ndarray:
    """Place `audio` (mono or stereo) on the stereo bus at `position` in [-1, +1]."""
    if audio.shape[1] == 1 and position == 0.0:
        return to_stereo(audio)  # the same arithmetic as the raw sum (D-008)
    if audio.shape[1] == 1:
        left, right = mono_pan_gains(position)
        x = audio[:, 0].astype(np.float64)
        return np.stack([x * left, x * right], axis=1).astype(np.float32)
    left, right = balance_gains(position)
    if left == 1.0 and right == 1.0:
        return audio.astype(np.float32, copy=False)
    gains = np.array([left, right], dtype=np.float64)
    return (audio.astype(np.float64) * gains).astype(np.float32)
