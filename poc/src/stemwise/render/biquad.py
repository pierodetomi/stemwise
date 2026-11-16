"""RBJ biquad filters (PRD §7.1, D-029): high-pass, low-pass, peak and shelving EQ.

Coefficients follow the Audio EQ Cookbook (R. Bristow-Johnson), Q form, normalised by `a0`; they
depend only on the plan parameters and the sample rate. Filtering runs in float64 with
`scipy.signal.sosfilt`, from a zero initial state. Formulas: docs/analysis.md, "Render".
"""

from __future__ import annotations

import math

import numpy as np
from scipy.signal import sosfilt

BUTTERWORTH_Q = 1.0 / math.sqrt(2.0)  # second-order Butterworth: 12 dB/octave, -3.01 dB at f0


def _intermediates(freq_hz: float, q: float, sample_rate: int) -> tuple[float, float]:
    if not 0.0 < freq_hz < sample_rate / 2.0:
        raise ValueError(f"frequency {freq_hz} Hz must be in (0, {sample_rate / 2} Hz)")
    if q <= 0.0:
        raise ValueError(f"q must be positive, got {q}")
    w0 = 2.0 * math.pi * freq_hz / sample_rate
    return math.cos(w0), math.sin(w0) / (2.0 * q)


def _sos(b0: float, b1: float, b2: float, a0: float, a1: float, a2: float) -> np.ndarray:
    return np.array([[b0 / a0, b1 / a0, b2 / a0, 1.0, a1 / a0, a2 / a0]], dtype=np.float64)


def highpass(freq_hz: float, sample_rate: int, q: float = BUTTERWORTH_Q) -> np.ndarray:
    cos, alpha = _intermediates(freq_hz, q, sample_rate)
    return _sos((1 + cos) / 2, -(1 + cos), (1 + cos) / 2, 1 + alpha, -2 * cos, 1 - alpha)


def lowpass(freq_hz: float, sample_rate: int, q: float = BUTTERWORTH_Q) -> np.ndarray:
    cos, alpha = _intermediates(freq_hz, q, sample_rate)
    return _sos((1 - cos) / 2, 1 - cos, (1 - cos) / 2, 1 + alpha, -2 * cos, 1 - alpha)


def peak(freq_hz: float, gain_db: float, q: float, sample_rate: int) -> np.ndarray:
    cos, alpha = _intermediates(freq_hz, q, sample_rate)
    a = 10.0 ** (gain_db / 40.0)
    return _sos(1 + alpha * a, -2 * cos, 1 - alpha * a, 1 + alpha / a, -2 * cos, 1 - alpha / a)


def low_shelf(freq_hz: float, gain_db: float, q: float, sample_rate: int) -> np.ndarray:
    cos, alpha = _intermediates(freq_hz, q, sample_rate)
    a = 10.0 ** (gain_db / 40.0)
    k = 2.0 * math.sqrt(a) * alpha
    return _sos(
        a * ((a + 1) - (a - 1) * cos + k),
        2 * a * ((a - 1) - (a + 1) * cos),
        a * ((a + 1) - (a - 1) * cos - k),
        (a + 1) + (a - 1) * cos + k,
        -2 * ((a - 1) + (a + 1) * cos),
        (a + 1) + (a - 1) * cos - k,
    )


def high_shelf(freq_hz: float, gain_db: float, q: float, sample_rate: int) -> np.ndarray:
    cos, alpha = _intermediates(freq_hz, q, sample_rate)
    a = 10.0 ** (gain_db / 40.0)
    k = 2.0 * math.sqrt(a) * alpha
    return _sos(
        a * ((a + 1) + (a - 1) * cos + k),
        -2 * a * ((a - 1) + (a + 1) * cos),
        a * ((a + 1) + (a - 1) * cos - k),
        (a + 1) - (a - 1) * cos + k,
        2 * ((a - 1) - (a + 1) * cos),
        (a + 1) - (a - 1) * cos - k,
    )


EQ_DESIGNS = {"peak": peak, "low_shelf": low_shelf, "high_shelf": high_shelf}


def eq_band(kind: str, freq_hz: float, gain_db: float, q: float, sample_rate: int) -> np.ndarray:
    return EQ_DESIGNS[kind](freq_hz, gain_db, q, sample_rate)


def apply(audio: np.ndarray, sos: np.ndarray) -> np.ndarray:
    """Filter every channel of `audio` (frames, channels); float64 inside, float32 out."""
    out = sosfilt(sos, np.asarray(audio, dtype=np.float64), axis=0)
    return out.astype(np.float32)
