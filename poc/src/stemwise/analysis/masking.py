"""Masking between stems (ANA-9, D-016). Formulas: docs/analysis.md, "Mascheramento".

For every pair of stems and every band, frame by frame on the shared STFT grid:
`score = Σ_{both active} min(E_a, E_b) / Σ_{at least one active} max(E_a, E_b)`, in [0, 1], where
E is the band power the stem brings to the bus and "active in a band" means a band level above
-50 dBFS. Identical stems score exactly 1.
"""

from __future__ import annotations

from dataclasses import dataclass
from itertools import combinations

import numpy as np

from stemwise.analysis.spectral import ACTIVITY_THRESHOLD_POWER, Spectrum

TOP_COLLISIONS = 10


@dataclass(frozen=True)
class Collision:
    a: str
    b: str
    band: int  # index into spectral.BANDS_HZ / BAND_LABELS
    score: float


def pair_scores(spec_a: Spectrum, spec_b: Spectrum) -> list[float]:
    """Masking score of two stems in each of the 8 bands."""
    if spec_a.n_frames != spec_b.n_frames:
        raise ValueError("masking needs stems of equal length on the same frame grid")
    active_a = spec_a.band_power > ACTIVITY_THRESHOLD_POWER
    active_b = spec_b.band_power > ACTIVITY_THRESHOLD_POWER
    energy_a, energy_b = spec_a.bus_band_power, spec_b.bus_band_power
    both, either = active_a & active_b, active_a | active_b
    lower = np.where(both, np.minimum(energy_a, energy_b), 0.0).sum(axis=0)
    upper = np.where(either, np.maximum(energy_a, energy_b), 0.0).sum(axis=0)
    return [float(lo / up) if up > 0 else 0.0 for lo, up in zip(lower, upper, strict=True)]


def collisions(stems: dict[str, Spectrum]) -> list[Collision]:
    """Every pair (in project order) and band with a score above 0, strongest first.

    Ties keep project order of the pair, then band order, so the ranking is deterministic.
    """
    found = [
        Collision(a, b, band, score)
        for a, b in combinations(stems, 2)
        for band, score in enumerate(pair_scores(stems[a], stems[b]))
        if score > 0.0
    ]
    return sorted(found, key=lambda c: -c.score)  # sorted() is stable


def masking_top(stems: dict[str, Spectrum], n: int = TOP_COLLISIONS) -> list[Collision]:
    """ANA-9: the `n` strongest collisions."""
    return collisions(stems)[:n]
