"""Mix bus: stereo summing, true-peak trim, raw sum (REN-5, REN-7, D-007, D-008)."""

from __future__ import annotations

import hashlib
import math
from pathlib import Path

import numpy as np
import pytest

from stemwise.analysis.stem import true_peak_db
from stemwise.audio_io import Stems, read_audio, write_wav24
from stemwise.render.mixbus import (
    MONO_CENTER_GAIN,
    TRIM_MARGIN_DB,
    render_baseline,
    sum_bus,
    to_stereo,
    trim,
)
from tests.helpers.signals import SR, fade, noise, sine

CEILING = -3.0


def _loud_stems() -> Stems:
    """Four in-phase stems whose sum peaks around +4 dBTP: the trim must kick in."""
    return Stems(
        SR,
        {
            "a": fade(sine(220, amplitude=0.5)),
            "b": fade(sine(220, amplitude=0.5, channels=2)),
            "c": fade(sine(220, amplitude=0.5, channels=2)),
            "d": fade(noise(amplitude=0.05, channels=2, seed=2)),
        },
    )


@pytest.mark.req("REN-7")
def test_mono_stem_centered_constant_power() -> None:
    x = noise(amplitude=0.2, seed=4)
    y = to_stereo(x)
    assert y.shape == (x.shape[0], 2)
    assert y.dtype == np.float32
    assert pytest.approx(math.sqrt(0.5)) == MONO_CENTER_GAIN
    # -3.01 dB per channel: total power of the stereo pair equals the power of the mono stem.
    np.testing.assert_allclose(y, np.hstack([x, x]) * math.sqrt(0.5), rtol=0, atol=1e-7)


@pytest.mark.req("REN-7")
def test_stereo_stem_passes_unchanged() -> None:
    x = noise(amplitude=0.2, channels=2, seed=4)
    np.testing.assert_array_equal(to_stereo(x), x)


@pytest.mark.req("REN-7")
def test_baseline_sums_stems() -> None:
    mono, stereo = noise(amplitude=0.1, seed=1), noise(amplitude=0.1, channels=2, seed=2)
    bus = sum_bus([mono, stereo])
    expected = np.hstack([mono, mono]) * math.sqrt(0.5) + stereo
    # 1e-6: float32 rounding of the sum, far below the 24-bit LSB (1.2e-7 · full scale ≈ 6e-8).
    np.testing.assert_allclose(bus, expected, rtol=0, atol=1e-6)
    assert bus.dtype == np.float32


@pytest.mark.req("REN-5")
def test_trim_attenuates_to_limit() -> None:
    bus = sum_bus(_loud_stems().audio.values())
    before = true_peak_db(bus)
    assert before > 0.0  # precondition: the raw sum clips
    trimmed, gain_db = trim(bus, CEILING)
    assert gain_db == pytest.approx(CEILING - TRIM_MARGIN_DB - before, abs=1e-9)
    after = true_peak_db(trimmed)
    assert -3.02 <= after <= -3.0
    # Pure gain: same waveform scaled.
    np.testing.assert_allclose(trimmed, bus * 10 ** (gain_db / 20), rtol=0, atol=1e-6)


@pytest.mark.req("REN-5")
def test_trim_leaves_quiet_sum_untouched() -> None:
    bus = sum_bus([fade(sine(220, amplitude=0.1, channels=2))])
    trimmed, gain_db = trim(bus, CEILING)
    assert gain_db == 0.0
    np.testing.assert_array_equal(trimmed, bus)


@pytest.mark.req("REN-5")
def test_trim_never_boosts() -> None:
    bus = sum_bus([fade(sine(220, amplitude=0.01, channels=2))])
    assert trim(bus, CEILING)[1] == 0.0


@pytest.mark.req("REN-5")
def test_trim_honours_stricter_ceiling() -> None:
    bus = sum_bus(_loud_stems().audio.values())
    trimmed, _ = trim(bus, -6.0)
    assert true_peak_db(trimmed) == pytest.approx(-6.0 - TRIM_MARGIN_DB, abs=1e-3)


@pytest.mark.req("REN-5", "REN-7")
def test_render_baseline_reports_levels() -> None:
    result = render_baseline(_loud_stems(), CEILING)
    assert result.audio.shape == (SR, 2)
    assert result.true_peak_before_db > 0.0
    assert result.trim_gain_db < 0.0
    assert -3.02 <= result.true_peak_db <= -3.0


@pytest.mark.req("REN-5", "REN-7")
def test_written_baseline_stays_within_ceiling(tmp_path: Path) -> None:
    result = render_baseline(_loud_stems(), CEILING)
    audio, _ = read_audio(write_wav24(tmp_path / "baseline.wav", result.audio, SR))
    assert true_peak_db(audio) <= CEILING  # 24-bit rounding does not cross the ceiling (D-007)


@pytest.mark.req("REN-7")
def test_baseline_file_is_deterministic(tmp_path: Path) -> None:
    digests = []
    for name in ("one.wav", "two.wav"):
        result = render_baseline(_loud_stems(), CEILING)
        path = write_wav24(tmp_path / name, result.audio, SR)
        digests.append(hashlib.sha256(path.read_bytes()).hexdigest())
    assert digests[0] == digests[1]
