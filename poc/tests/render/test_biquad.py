"""RBJ biquads (REN-2, D-029): analytic responses, sines through the filter, pedalboard EQ."""

from __future__ import annotations

import math

import numpy as np
import pedalboard
import pytest
from scipy.signal import sosfreqz

from stemwise.render import biquad
from tests.helpers.signals import sine

FS = 48_000


def _response_db(sos: np.ndarray, freqs: list[float] | np.ndarray, fs: int = FS) -> np.ndarray:
    _, h = sosfreqz(sos, worN=np.asarray(freqs, dtype=np.float64), fs=fs)
    return 20.0 * np.log10(np.abs(h))


def _sine_gain_db(sos: np.ndarray, freq: float, fs: int = FS) -> float:
    """Gain measured on a steady sine: RMS of the last second, an integer number of periods."""
    x = sine(freq, amplitude=0.5, sr=fs, duration_s=3.0)
    y = biquad.apply(x, sos)
    tail = slice(2 * fs, 3 * fs)
    return 20.0 * math.log10(np.sqrt(np.mean(y[tail] ** 2)) / np.sqrt(np.mean(x[tail] ** 2)))


@pytest.mark.req("REN-2")
@pytest.mark.parametrize("fs", [44_100, 48_000, 96_000])
def test_highpass_response(fs: int) -> None:
    sos = biquad.highpass(200.0, fs)
    r = _response_db(sos, [50.0, 200.0, 2_000.0, 10_000.0], fs)
    # Analytic 2nd-order Butterworth: |H|^2 = (f/fc)^4 / (1 + (f/fc)^4), bilinear warping aside.
    assert r[1] == pytest.approx(-3.01, abs=0.01)
    assert r[0] == pytest.approx(-24.1, abs=0.1)  # two octaves below: 12 dB/octave
    assert r[0] <= -23.0
    assert r[2] > -0.01
    assert abs(r[3]) < 0.01


@pytest.mark.req("REN-2")
@pytest.mark.parametrize("fs", [44_100, 48_000])
def test_lowpass_response(fs: int) -> None:
    sos = biquad.lowpass(4_000.0, fs)
    r = _response_db(sos, [100.0, 400.0, 4_000.0, 16_000.0], fs)
    assert r[2] == pytest.approx(-3.01, abs=0.01)
    assert abs(r[0]) < 0.01
    assert abs(r[1]) < 0.01
    assert r[3] <= -24.0  # two octaves above; the bilinear transform only deepens the cut


@pytest.mark.req("REN-2")
@pytest.mark.parametrize("gain_db", [6.0, -6.0, 9.0])
def test_peak_response(gain_db: float) -> None:
    sos = biquad.peak(1_000.0, gain_db, 1.4, FS)
    r = _response_db(sos, [20.0, 1_000.0, 20_000.0])
    assert r[1] == pytest.approx(gain_db, abs=0.01)
    assert abs(r[0]) < 0.05
    assert abs(r[2]) < 0.05
    # Monotonic away from the centre on both sides.
    sides = _response_db(sos, [250.0, 500.0, 1_000.0, 2_000.0, 4_000.0])
    mag = np.abs(sides)
    assert mag[0] < mag[1] < mag[2]
    assert mag[4] < mag[3] < mag[2]


@pytest.mark.req("REN-2")
@pytest.mark.parametrize(("kind", "far"), [("low_shelf", 20.0), ("high_shelf", 20_000.0)])
@pytest.mark.parametrize("gain_db", [6.0, -6.0])
def test_shelf_response(kind: str, far: float, gain_db: float) -> None:
    f0 = 200.0 if kind == "low_shelf" else 6_000.0
    sos = biquad.eq_band(kind, f0, gain_db, biquad.BUTTERWORTH_Q, FS)
    other = 20_000.0 if kind == "low_shelf" else 20.0
    r = _response_db(sos, [far, f0, other])
    assert r[1] == pytest.approx(gain_db / 2.0, abs=0.01)  # RBJ shelf: half the gain at f0
    assert r[0] == pytest.approx(gain_db, abs=0.1)
    assert abs(r[2]) < 0.05


@pytest.mark.req("REN-2")
@pytest.mark.parametrize(
    ("sos", "freq"),
    [
        (biquad.highpass(200.0, FS), 50.0),
        (biquad.highpass(200.0, FS), 200.0),
        (biquad.lowpass(4_000.0, FS), 8_000.0),
        (biquad.peak(1_000.0, 6.0, 1.4, FS), 1_000.0),
        (biquad.high_shelf(6_000.0, -6.0, 0.7, FS), 12_000.0),
    ],
)
def test_sine_through_filter_matches_response(sos: np.ndarray, freq: float) -> None:
    assert _sine_gain_db(sos, freq) == pytest.approx(_response_db(sos, [freq])[0], abs=0.1)


@pytest.mark.req("REN-2")
@pytest.mark.parametrize(
    ("kind", "plugin"),
    [
        ("peak", pedalboard.PeakFilter),
        ("low_shelf", pedalboard.LowShelfFilter),
        ("high_shelf", pedalboard.HighShelfFilter),
    ],
)
@pytest.mark.parametrize(("freq", "gain_db", "q"), [(300.0, -4.0, 1.4), (3_000.0, 6.0, 0.7)])
def test_biquads_match_pedalboard_eq(
    kind: str, plugin: type, freq: float, gain_db: float, q: float
) -> None:
    """Same RBJ formulas as pedalboard's (JUCE) EQ: the choice of D-029 changes no curve."""
    impulse = np.zeros((1, FS), dtype=np.float32)
    impulse[0, 0] = 1.0
    ir = plugin(cutoff_frequency_hz=freq, gain_db=gain_db, q=q).process(impulse, FS)[0]
    spectrum = np.abs(np.fft.rfft(ir.astype(np.float64)))
    bins = np.geomspace(30.0, 18_000.0, 20)
    theirs = 20.0 * np.log10(spectrum[np.round(bins).astype(int)])  # 1 Hz per bin (1 s IR)
    ours = _response_db(biquad.eq_band(kind, freq, gain_db, q, FS), np.round(bins))
    np.testing.assert_allclose(ours, theirs, atol=0.05)  # float32 vs float64


def test_apply_keeps_shape_and_dtype() -> None:
    x = sine(440.0, channels=2)
    y = biquad.apply(x, biquad.highpass(100.0, FS))
    assert y.shape == x.shape
    assert y.dtype == np.float32


@pytest.mark.parametrize("freq", [0.0, 24_000.0, 30_000.0])
def test_frequency_outside_range_rejected(freq: float) -> None:
    with pytest.raises(ValueError, match="frequency"):
        biquad.highpass(freq, FS)
