"""Masking between stems (ANA-9, D-016)."""

from __future__ import annotations

import numpy as np
import pytest

from stemwise.analysis.masking import Collision, collisions, masking_top, pair_scores
from stemwise.analysis.spectral import analyze_spectrum
from tests.helpers.signals import SR, pink_noise, silence, sine


def _spec(x: np.ndarray):
    return analyze_spectrum(x, SR)


@pytest.mark.req("ANA-9")
def test_identical_stems_score_one() -> None:
    x = pink_noise(duration_s=10.0, channels=2)
    scores = pair_scores(_spec(x), _spec(x.copy()))
    np.testing.assert_allclose(scores, 1.0, atol=1e-6)


@pytest.mark.req("ANA-9")
def test_quieter_copy_score() -> None:
    # Same signal 12 dB lower, active in the same frames: min/max = 10^(-12/10) in every band.
    x = pink_noise(duration_s=10.0, rms=0.1)
    scores = pair_scores(_spec(x), _spec(x * 10 ** (-12 / 20)))
    np.testing.assert_allclose(scores, 10 ** (-1.2), rtol=0.01)


@pytest.mark.req("ANA-9")
def test_disjoint_stems_score_zero() -> None:
    first = np.concatenate([pink_noise(duration_s=5.0), silence(duration_s=5.0)])
    second = np.concatenate([silence(duration_s=5.0), pink_noise(duration_s=5.0, seed=1)])
    # One frame (~170 ms) may straddle the switch: bound the overlap instead of requiring 0.
    assert max(pair_scores(_spec(first), _spec(second))) < 0.02
    low, high = sine(100, duration_s=5.0), sine(5000, duration_s=5.0)
    assert pair_scores(_spec(low), _spec(high)) == [0.0] * 8


@pytest.mark.req("ANA-9")
def test_top_ten_order() -> None:
    stems = {
        f"s{i}": _spec(pink_noise(duration_s=3.0, seed=i, rms=0.05 * (i + 1))) for i in range(6)
    }
    top = masking_top(stems)
    every = collisions(stems)
    assert len(every) == 15 * 8  # 15 pairs, every band active
    assert len(top) == 10
    assert top == every[:10]
    scores = [c.score for c in every]
    assert scores == sorted(scores, reverse=True)
    assert all(isinstance(c, Collision) and 0.0 < c.score <= 1.0 for c in top)


@pytest.mark.req("ANA-9")
def test_ties_keep_project_and_band_order() -> None:
    x = pink_noise(duration_s=5.0)
    stems = {"c": _spec(x), "a": _spec(x.copy()), "b": _spec(x.copy())}
    top = masking_top(stems)
    # All three pairs score 1 in all 8 bands: ties ordered by pair (project order), then band.
    assert [(c.a, c.b, c.band) for c in top] == [
        *[("c", "a", band) for band in range(8)],
        ("c", "b", 0),
        ("c", "b", 1),
    ]


def test_mismatched_grids_rejected() -> None:
    with pytest.raises(ValueError, match="equal length"):
        pair_scores(_spec(sine(100, duration_s=1.0)), _spec(sine(100, duration_s=2.0)))
