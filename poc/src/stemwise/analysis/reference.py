"""Reference tracks (ANA-11) and the target profile (ANA-12). Formulas: docs/analysis.md.

A reference is analysed as a stereo mix (the measures of ANA-7), after being brought to
REFERENCE_LUFS, so the comparison does not depend on its playback level. References arrive
already resampled to the project rate (ING-5).
"""

from __future__ import annotations

import math
from dataclasses import dataclass

import numpy as np

from stemwise.analysis.loudness import gain_to_lufs, integrated_lufs
from stemwise.analysis.mix import MixMeasures, measure_mix
from stemwise.analysis.spectral import N_BANDS

REFERENCE_LUFS = -23.0  # same level as the stem pre-gain of REN-1


class ReferenceLoudnessError(ValueError):
    """The reference has no measurable loudness (silent, or shorter than one gating block)."""


def normalize_reference(audio: np.ndarray, sample_rate: int) -> np.ndarray:
    """Scale `audio` so that its integrated loudness is REFERENCE_LUFS (float64)."""
    lufs = integrated_lufs(audio, sample_rate)
    if lufs is None or not math.isfinite(lufs):
        raise ReferenceLoudnessError("silent or shorter than 0.4 s: loudness cannot be normalized")
    return np.asarray(audio, dtype=np.float64) * gain_to_lufs(lufs, REFERENCE_LUFS)


def measure_reference(audio: np.ndarray, sample_rate: int) -> MixMeasures:
    """ANA-11: the measures of ANA-7 on the loudness-normalized reference."""
    return measure_mix(normalize_reference(audio, sample_rate), sample_rate)


@dataclass(frozen=True)
class Target:
    """ANA-12: mean profile of the references; a field is None if no reference contributes."""

    n_references: int
    bands_db: list[float] | None
    side_mid_db: float | None
    dyn_range_lu: float | None


def _mean(values: list[float]) -> float | None:
    finite = [v for v in values if math.isfinite(v)]
    return sum(finite) / len(finite) if finite else None


def target_profile(references: list[MixMeasures]) -> Target | None:
    """ANA-12: arithmetic mean of the reference values; None without references.

    Bands are averaged in dB, band by band. A reference contributes to a field only where it has
    a finite value: mono (and dual-mono) references do not enter the side/mid mean.
    """
    if not references:
        return None
    bands = [r.bands_db for r in references if r.bands_db is not None]
    mean_bands = [sum(b[i] for b in bands) / len(bands) for i in range(N_BANDS)] if bands else None
    return Target(
        n_references=len(references),
        bands_db=mean_bands,
        side_mid_db=_mean(
            [
                r.stereo.side_mid_db
                for r in references
                if r.stereo and r.stereo.side_mid_db is not None
            ]
        ),
        dyn_range_lu=_mean([r.dyn_range_lu for r in references if r.dyn_range_lu is not None]),
    )
