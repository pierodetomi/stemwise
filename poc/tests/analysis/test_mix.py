"""Mix measures (ANA-7), band shares (ANA-8, D-017), low-frequency correlation (ANA-10)."""

from __future__ import annotations

import math

import numpy as np
import pytest

from stemwise.analysis.loudness import integrated_lufs, loudness_range_lu
from stemwise.analysis.mix import band_share_pct, measure_mix
from stemwise.analysis.spectral import analyze_spectrum, band_levels_db
from stemwise.analysis.stem import sample_peak_db, true_peak_db
from stemwise.render.mixbus import sum_bus
from tests.helpers.signals import SR, noise, pink_noise, sine


@pytest.mark.req("ANA-7", "ANA-10")
def test_mix_measures_use_stem_formulas() -> None:
    mix = sum_bus([pink_noise(duration_s=10.0, channels=2), sine(100, duration_s=10.0)])
    m = measure_mix(mix, SR)
    assert m.lufs == integrated_lufs(mix, SR)
    assert m.dyn_range_lu == loudness_range_lu(mix, SR)
    assert m.peak_db == sample_peak_db(mix)
    assert m.true_peak_db == true_peak_db(mix)
    assert m.bands_db == band_levels_db(analyze_spectrum(mix, SR))
    assert m.rms_db is not None
    assert m.crest_db == pytest.approx(m.peak_db - m.rms_db)
    assert m.stereo is not None
    assert m.stereo.correlation is not None
    assert m.low_corr is not None


def _shares(*stems: np.ndarray) -> dict[str, list[float]]:
    return band_share_pct({f"s{i}": analyze_spectrum(x, SR) for i, x in enumerate(stems)})


@pytest.mark.req("ANA-8")
def test_band_share_disjoint_sines() -> None:
    shares = _shares(sine(100, duration_s=5.0), sine(1000, duration_s=5.0))
    # ±0.1 points: leakage of each sine into the other's band is below -60 dB.
    assert shares["s0"][1] == pytest.approx(100.0, abs=0.1)  # 60-120 Hz
    assert shares["s1"][4] == pytest.approx(100.0, abs=0.1)  # 500-2k
    assert shares["s0"][4] == pytest.approx(0.0, abs=0.1)
    assert shares["s1"][1] == pytest.approx(0.0, abs=0.1)


@pytest.mark.req("ANA-8")
def test_band_share_equal_noises() -> None:
    shares = _shares(noise(duration_s=30.0, seed=1), noise(duration_s=30.0, seed=2))
    # ±1 point: two independent estimates of the same band power over 30 s.
    np.testing.assert_allclose(shares["s0"], 50.0, atol=1.0)  # measured: max 0.8 off
    np.testing.assert_allclose(shares["s1"], 50.0, atol=1.0)


@pytest.mark.req("ANA-8")
def test_band_share_mono_vs_stereo_on_bus() -> None:
    # D-008: a mono stem x and a stereo stem L = R = x·√½ bring the same power to the bus.
    x = pink_noise(duration_s=5.0)
    stereo = np.repeat(x * math.sqrt(0.5), 2, axis=1)
    shares = _shares(x, stereo)
    np.testing.assert_allclose(shares["s0"], 50.0, atol=0.1)


@pytest.mark.req("ANA-8")
def test_band_shares_sum_to_100_or_zero() -> None:
    shares = _shares(sine(100, duration_s=5.0), pink_noise(duration_s=5.0, channels=2))
    totals = np.sum(list(shares.values()), axis=0)
    np.testing.assert_allclose(totals, 100.0, atol=0.01)
    empty = _shares(np.zeros((SR, 1), np.float32), np.zeros((SR, 2), np.float32))
    assert empty == {"s0": [0.0] * 8, "s1": [0.0] * 8}


def _stereo(left: np.ndarray, right: np.ndarray) -> np.ndarray:
    return np.concatenate([left, right], axis=1)


@pytest.mark.req("ANA-10")
@pytest.mark.parametrize(
    ("mix", "expected"),
    [
        # In-phase 60 Hz with anti-phase 1 kHz: only the lows count.
        (
            _stereo(
                sine(60, duration_s=5.0) + sine(1000, duration_s=5.0),
                sine(60, duration_s=5.0) - sine(1000, duration_s=5.0),
            ),
            1.0,
        ),
        (_stereo(sine(60, duration_s=5.0), -sine(60, duration_s=5.0)), -1.0),
        (sum_bus([sine(60, duration_s=5.0), sine(80, duration_s=5.0)]), 1.0),
    ],
    ids=["lows-in-phase", "lows-anti-phase", "mono-stems"],
)
def test_low_correlation(mix: np.ndarray, expected: float) -> None:
    assert measure_mix(mix, SR).low_corr == pytest.approx(expected, abs=0.01)
