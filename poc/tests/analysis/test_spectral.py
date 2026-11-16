"""Analysis STFT: grid, activity (D-014), 8 bands (ANA-3), active time and centroid (ANA-4)."""

from __future__ import annotations

import math

import numpy as np
import pytest

from stemwise.analysis.spectral import (
    BANDS_HZ,
    active_pct,
    analyze_spectrum,
    band_index,
    band_levels_db,
    centroid_hz,
    fft_size,
    frame_count,
)
from stemwise.analysis.stem import noise_floor_db, rms_db
from tests.helpers.signals import SR, noise, pink_noise, silence, sine

BAND_500_2K = 4


def test_fft_size_keeps_frame_duration() -> None:
    assert fft_size(44_100) == 8192
    assert fft_size(48_000) == 8192
    assert fft_size(88_200) == 16384
    assert fft_size(96_000) == 16384


def test_frame_count() -> None:
    assert frame_count(0, 8192, 2048) == 0
    assert frame_count(100, 8192, 2048) == 1
    assert frame_count(8192, 8192, 2048) == 1
    assert frame_count(8193, 8192, 2048) == 2


def test_frame_power_is_mean_square() -> None:
    # Parseval: the bin powers of a frame sum to its Hann-weighted mean square (0.5·A² for a sine).
    spec = analyze_spectrum(sine(997, amplitude=0.5, duration_s=2.0), SR)
    inner = spec.frame_power[1:-4]  # frames fully inside the signal
    np.testing.assert_allclose(inner, 0.125, rtol=1e-3)  # windowed non-integer periods: < 0.1%
    np.testing.assert_allclose(spec.band_power.sum(axis=1)[1:-4], inner, rtol=1e-6)


def test_band_index_edges() -> None:
    freqs = np.array([19.9, 20.0, 59.9, 60.0, 1999.9, 2000.0, 19999.0, 20000.0, 20000.1])
    np.testing.assert_array_equal(band_index(freqs, 48_000), [-1, 0, 0, 1, 4, 5, 7, 7, -1])


@pytest.mark.req("ANA-3")
def test_sine_band() -> None:
    bands = band_levels_db(analyze_spectrum(sine(1000, duration_s=3.0), SR))
    assert bands is not None
    # Nearest band edge (500 Hz) is 85 bins away: Hann leakage there is far below -100 dB.
    assert bands[BAND_500_2K] >= -0.05  # ≥ 98.9% of the energy
    assert all(b <= -60.0 for i, b in enumerate(bands) if i != BAND_500_2K)


@pytest.mark.req("ANA-3")
@pytest.mark.parametrize(("freq", "band"), [(1950.0, 4), (2050.0, 5), (110.0, 1), (130.0, 2)])
def test_band_edges(freq: float, band: int) -> None:
    # Bins at f with lo <= f < hi. 50 Hz and 10 Hz from an edge the leakage into the neighbour band
    # (sidelobes and the abrupt start/end of the sine) measures -51 and -33 dB: ≥ 99.8% stays.
    bands = band_levels_db(analyze_spectrum(sine(freq, duration_s=3.0), SR))
    assert bands is not None
    assert bands[band] >= -0.01


@pytest.mark.req("ANA-3")
def test_bands_sum_to_total() -> None:
    bands = band_levels_db(analyze_spectrum(pink_noise(duration_s=5.0, channels=2), SR))
    assert bands is not None
    assert sum(10 ** (b / 10) for b in bands) == pytest.approx(1.0, abs=1e-9)


@pytest.mark.req("ANA-3")
def test_pink_noise_band_energy_per_octave() -> None:
    bands = band_levels_db(analyze_spectrum(pink_noise(duration_s=30.0), SR))
    assert bands is not None
    expected = [10 * math.log10(math.log2(hi / lo) / math.log2(1000)) for lo, hi in BANDS_HZ]
    # Exact 1/f magnitude: deviations come only from STFT edge leakage and frame averaging.
    np.testing.assert_allclose(bands, expected, atol=0.5)
    one_octave = [bands[i] for i in (1, 3, 5, 6)]  # 60-120, 250-500, 2k-4k, 4k-8k
    assert max(one_octave) - min(one_octave) <= 0.3


@pytest.mark.req("ANA-3", "ANA-4")
def test_bands_ignore_inactive_frames() -> None:
    # 1 kHz sine, then 5 s of white noise at -60 dBFS RMS (inactive, below -50 dBFS).
    x = np.concatenate([sine(1000, duration_s=3.0), noise(duration_s=5.0, amplitude=1e-3)])
    bands = band_levels_db(analyze_spectrum(x, SR))
    assert bands is not None
    assert bands[BAND_500_2K] >= -0.05


@pytest.mark.req("ANA-4")
def test_active_pct() -> None:
    x = np.concatenate([sine(1000, duration_s=3.0), silence(duration_s=7.0)])
    # ±2 points: a frame (~170 ms, 1.7% of 10 s) straddling the edge may still be active.
    assert active_pct(analyze_spectrum(x, SR)) == pytest.approx(30.0, abs=2.0)
    assert active_pct(analyze_spectrum(silence(duration_s=2.0), SR)) == 0.0
    assert active_pct(analyze_spectrum(sine(1000, amplitude=1e-3, duration_s=2.0), SR)) == 0.0


@pytest.mark.req("ANA-4")
def test_centroid() -> None:
    assert centroid_hz(analyze_spectrum(sine(1000, duration_s=3.0), SR)) == pytest.approx(
        1000.0,
        abs=6.0,  # one bin (5.9 Hz)
    )
    two = sine(500, duration_s=3.0, amplitude=0.25) + sine(2000, duration_s=3.0, amplitude=0.25)
    # Power-weighted: (500 + 2000) / 2.
    assert centroid_hz(analyze_spectrum(two, SR)) == pytest.approx(1250.0, abs=10.0)


def test_silence_has_no_bands_or_centroid() -> None:
    spec = analyze_spectrum(silence(duration_s=2.0, channels=2), SR)
    assert band_levels_db(spec) is None
    assert centroid_hz(spec) is None
    empty = analyze_spectrum(np.zeros((0, 1), dtype=np.float32), SR)
    assert empty.n_frames == 0
    assert active_pct(empty) == 0.0


def test_mono_and_dual_mono_agree() -> None:
    x = pink_noise(duration_s=5.0)
    mono = analyze_spectrum(x, SR)
    dual = analyze_spectrum(np.repeat(x, 2, axis=1), SR)
    np.testing.assert_allclose(dual.frame_power, mono.frame_power, rtol=1e-12)
    np.testing.assert_allclose(dual.bus_band_power, 2 * mono.bus_band_power, rtol=1e-12)


@pytest.mark.req("ANA-1", "ANA-4", "ANA-6")
def test_activity_ignores_dc_and_rumble() -> None:
    # D-022: frame power is limited to 20 Hz - 20 kHz. Before, a DC of 0.4% (above -50 dBFS)
    # made every frame active: active_pct 100 instead of 30, RMS -18.7 instead of -13.5 dBFS.
    tone = np.concatenate([sine(997, amplitude=0.3, duration_s=3.0), silence(duration_s=7.0)])
    clean, offset = analyze_spectrum(tone, SR), analyze_spectrum(tone + 0.004, SR)
    assert active_pct(offset) == active_pct(clean)
    assert rms_db(offset) == pytest.approx(rms_db(clean), abs=0.01)
    assert noise_floor_db(offset) < -80.0
    rumble = sine(10, amplitude=0.05, duration_s=5.0)  # -29 dBFS at 10 Hz
    assert active_pct(analyze_spectrum(rumble, SR)) == 0.0
