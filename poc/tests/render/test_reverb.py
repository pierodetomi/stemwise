"""Shared reverb (REN-4, D-031)."""

from __future__ import annotations

import numpy as np
import pytest

from stemwise.decision.schema import Reverb
from stemwise.render.reverb import render_return
from tests.helpers.signals import SR, sine


def _params(**overrides: float) -> Reverb:
    base = {"room_size": 0.5, "damping": 0.5, "return_hpf_hz": 20.0, "return_gain_db": 0.0}
    return Reverb.model_validate({**base, **overrides})


def _impulse(at_s: float = 0.5, duration_s: float = 3.0) -> np.ndarray:
    x = np.zeros((round(duration_s * SR), 2), dtype=np.float32)
    x[round(at_s * SR)] = 1.0
    return x


def _rms_db(x: np.ndarray) -> float:
    return float(10.0 * np.log10(np.mean(x.astype(np.float64) ** 2)))


@pytest.mark.req("REN-4")
def test_reverb_is_fully_wet() -> None:
    """No dry path: nothing before the shortest comb delay (≈25 ms), then a decaying tail."""
    y = render_return(_impulse(), _params(), SR)
    start = round(0.5 * SR)
    assert np.max(np.abs(y[:start])) == 0.0
    assert np.max(np.abs(y[start : start + round(0.005 * SR)])) < 1e-5  # < -100 dBFS
    assert np.max(np.abs(y[start + round(0.03 * SR) : start + SR])) > 1e-3  # the tail is there


@pytest.mark.req("REN-4")
def test_return_highpass() -> None:
    """60 Hz in the send: a 400 Hz high-pass on the return removes ≥ 25 dB (33 dB analytic)."""
    x = sine(60.0, amplitude=0.3, duration_s=3.0, channels=2)
    open_ = render_return(x, _params(damping=0.0, return_hpf_hz=20.0), SR)
    cut = render_return(x, _params(damping=0.0, return_hpf_hz=400.0), SR)
    steady = slice(SR, 3 * SR)
    assert _rms_db(open_[steady]) - _rms_db(cut[steady]) >= 25.0


@pytest.mark.req("REN-4")
def test_return_gain() -> None:
    x = _impulse()
    a = render_return(x, _params(return_gain_db=0.0), SR).astype(np.float64)
    b = render_return(x, _params(return_gain_db=-6.0), SR).astype(np.float64)
    np.testing.assert_allclose(b, a * 10 ** (-6 / 20), rtol=1e-6, atol=1e-12)


@pytest.mark.req("REN-4")
def test_reverb_keeps_duration_and_is_deterministic() -> None:
    x = _impulse(at_s=2.9, duration_s=3.0)  # the tail would run past the end: it is cut (D-031)
    a = render_return(x, _params(room_size=0.9), SR)
    b = render_return(x, _params(room_size=0.9), SR)
    assert a.shape == x.shape
    assert a.dtype == np.float32
    np.testing.assert_array_equal(a, b)


@pytest.mark.req("REN-4")
def test_reverb_input_is_summed_to_mono() -> None:
    """Freeverb sums L and R before the combs: the send position (pan) does not move the tail."""
    left = _impulse()
    left[:, 1] = 0.0
    right = _impulse()
    right[:, 0] = 0.0
    np.testing.assert_array_equal(
        render_return(left, _params(), SR), render_return(right, _params(), SR)
    )
