"""Integrated loudness (ANA-1) and loudness range (ANA-2, D-015)."""

from __future__ import annotations

import math

import numpy as np
import pytest

from stemwise.analysis.loudness import integrated_lufs, loudness_range_lu
from tests.helpers.signals import SR, pink_noise, silence, sine

# ±0.1 LU: compliance tolerance of EBU Tech 3341 for integrated loudness.
TOL_LUFS = 0.1
# ±1 LU: tolerance of the EBU Tech 3342 loudness-range test cases.
TOL_LRA = 1.0


def _db(level_dbfs: float) -> float:
    return 10 ** (level_dbfs / 20)


@pytest.mark.req("ANA-1")
def test_integrated_lufs_sine() -> None:
    # BS.1770: a 997 Hz sine at 0 dBFS on one channel reads -3.01 LUFS; 0.1 = -20 dBFS.
    mono = sine(997, amplitude=0.1, duration_s=10.0)
    assert integrated_lufs(mono, SR) == pytest.approx(-23.01, abs=TOL_LUFS)
    stereo = sine(997, amplitude=0.1, duration_s=10.0, channels=2)
    assert integrated_lufs(stereo, SR) == pytest.approx(-20.0, abs=TOL_LUFS)


@pytest.mark.req("ANA-1")
def test_integrated_lufs_gates_silence() -> None:
    tone = sine(997, amplitude=0.1, duration_s=10.0)
    gapped = np.concatenate([tone, silence(duration_s=10.0)])
    assert integrated_lufs(gapped, SR) == pytest.approx(integrated_lufs(tone, SR), abs=TOL_LUFS)


def test_integrated_lufs_edge_cases() -> None:
    assert integrated_lufs(silence(duration_s=2.0), SR) == -math.inf
    assert integrated_lufs(sine(997, duration_s=0.3), SR) is None


@pytest.mark.req("ANA-2")
@pytest.mark.parametrize(
    ("levels_dbfs", "expected_lu"),
    [
        ((-20, -30), 10.0),
        ((-20, -15), 5.0),
        ((-40, -20), 20.0),
        ((-50, -35, -20, -35, -50), 15.0),
    ],
)
def test_loudness_range_tech3342(levels_dbfs: tuple[int, ...], expected_lu: float) -> None:
    # EBU Tech 3342 test signals: stereo 1 kHz sine, 20 s at each level.
    x = np.concatenate(
        [sine(1000, amplitude=_db(level), duration_s=20.0, channels=2) for level in levels_dbfs]
    )
    assert loudness_range_lu(x, SR) == pytest.approx(expected_lu, abs=TOL_LRA)


@pytest.mark.req("ANA-2")
def test_loudness_range_stationary() -> None:
    assert loudness_range_lu(pink_noise(duration_s=30.0), SR) < 1.0


def test_loudness_range_edge_cases() -> None:
    assert loudness_range_lu(silence(duration_s=5.0), SR) is None
    assert loudness_range_lu(sine(1000, duration_s=1.0), SR) is None
