"""Reference analysis after loudness normalization (ANA-11) and target profile (ANA-12)."""

from __future__ import annotations

from dataclasses import replace
from pathlib import Path

import numpy as np
import pytest

from stemwise.analysis.loudness import integrated_lufs
from stemwise.analysis.mix import measure_mix
from stemwise.analysis.reference import (
    REFERENCE_LUFS,
    ReferenceLoudnessError,
    measure_reference,
    normalize_reference,
    target_profile,
)
from stemwise.analysis.stem import StereoMeasures
from stemwise.audio_io import load_reference
from tests.helpers.project import write_audio
from tests.helpers.signals import SR, noise, pink_noise, silence, sine

# ±0.05: gain does not change relative bands, side/mid or LRA; only float rounding and the
# discrete gating of the short-term loudness can move them.
TOL = 0.05


def _mix() -> np.ndarray:
    x = pink_noise(duration_s=20.0, channels=2, rms=0.2) + sine(100, duration_s=20.0, channels=2)
    return x + noise(duration_s=20.0, channels=2, amplitude=0.02, seed=4)


@pytest.mark.req("ANA-11")
def test_reference_gain_invariance() -> None:
    loud = _mix()
    quiet = (loud * 10 ** (-12 / 20)).astype(np.float32)
    a, b = measure_reference(loud, SR), measure_reference(quiet, SR)
    raw = measure_mix(loud, SR)
    for m in (a, b):
        assert m.bands_db is not None
        np.testing.assert_allclose(m.bands_db, raw.bands_db, atol=TOL)
        assert m.stereo is not None
        assert raw.stereo is not None
        assert m.stereo.side_mid_db == pytest.approx(raw.stereo.side_mid_db, abs=TOL)
        assert m.dyn_range_lu == pytest.approx(raw.dyn_range_lu, abs=TOL)


@pytest.mark.req("ANA-11")
def test_reference_normalized_lufs() -> None:
    m = measure_reference(_mix(), SR)
    assert m.lufs == pytest.approx(REFERENCE_LUFS, abs=TOL)
    assert integrated_lufs(normalize_reference(_mix(), SR), SR) == pytest.approx(-23.0, abs=TOL)


@pytest.mark.req("ANA-11")
def test_reference_resampled_bands(tmp_path: Path) -> None:
    path = write_audio(
        tmp_path / "ref.wav", sine(1000, sr=44_100, duration_s=5.0, channels=2), 44_100
    )
    m = measure_reference(load_reference(path, SR), SR)
    assert m.bands_db is not None
    assert m.bands_db[4] >= -0.05  # still in 500-2k after resampling to 48 kHz


def test_silent_reference_rejected() -> None:
    with pytest.raises(ReferenceLoudnessError):
        measure_reference(silence(duration_s=2.0, channels=2), SR)


@pytest.mark.req("ANA-12")
def test_target_is_mean() -> None:
    a = measure_reference(_mix(), SR)
    b = measure_reference(pink_noise(duration_s=10.0, channels=2, seed=7), SR)
    target = target_profile([a, b])
    assert target is not None
    assert target.n_references == 2
    assert a.bands_db is not None
    assert b.bands_db is not None
    np.testing.assert_allclose(
        target.bands_db, (np.array(a.bands_db) + np.array(b.bands_db)) / 2, atol=1e-9
    )
    assert a.stereo is not None
    assert b.stereo is not None
    assert target.side_mid_db == pytest.approx(
        (a.stereo.side_mid_db + b.stereo.side_mid_db) / 2, abs=1e-9
    )
    assert target.dyn_range_lu == pytest.approx((a.dyn_range_lu + b.dyn_range_lu) / 2, abs=1e-9)
    single = target_profile([a])
    assert single is not None
    assert (single.bands_db, single.dyn_range_lu) == (a.bands_db, a.dyn_range_lu)


@pytest.mark.req("ANA-12")
def test_no_references_no_target() -> None:
    assert target_profile([]) is None


@pytest.mark.req("ANA-12")
def test_mono_reference_side_mid() -> None:
    stereo = measure_reference(_mix(), SR)
    mono = measure_reference(pink_noise(duration_s=10.0), SR)
    dual = replace(stereo, stereo=StereoMeasures(correlation=1.0, side_mid_db=float("-inf")))
    assert mono.stereo is None
    target = target_profile([stereo, mono, dual])
    assert target is not None
    assert target.n_references == 3
    assert stereo.stereo is not None
    assert target.side_mid_db == pytest.approx(stereo.stereo.side_mid_db, abs=1e-9)
    only_mono = target_profile([mono])
    assert only_mono is not None
    assert only_mono.side_mid_db is None
