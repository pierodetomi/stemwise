"""Evaluation of a rendered mix (EVA-1 → EVA-4) and the contracts of the loop (D-049).

`Evaluation` is the contract of `evaluation.json` (PRD §6.4) and of `baseline_evaluation.json`
(the raw sum, `iteration: null`); it adds to §6.4 a `mix` block with the measures of ANA-7 and
ANA-10, in the format of the `mix` block of `analysis.json`. `Best` is the contract of
`best.json`, the final word on the best iteration (EVA-6). Formulas: docs/analysis.md,
"Valutazione".
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Annotated, Literal

import numpy as np
from pydantic import BaseModel, ConfigDict, Field

from stemwise.analysis.masking import Collision, masking_top
from stemwise.analysis.mix import MixMeasures, measure_mix
from stemwise.analysis.model import (
    Bands,
    CollisionEntry,
    Db,
    MixEntry,
    collision_entries,
    dumps_document,
    limited_db,
    mix_entry,
    rounded,
)
from stemwise.analysis.reference import Target
from stemwise.analysis.spectral import Spectrum
from stemwise.analysis.stem import dc_offset_pct

DC_LIMIT_PCT = 0.1  # EVA-4: "negligible" DC offset, percent of full scale (-60 dBFS), D-053

StopReason = Literal[
    "below_threshold",  # distance below the threshold of EVA-5
    "no_improvement",  # improved less than the minimum of EVA-5 (or got worse)
    "max_iterations",  # the maximum number of iterations was reached
    "no_target",  # no references: stopped at the first iteration within the constraints (D-052)
    "decision_failed",  # the decision of the next iteration failed (D-055)
    "revision",  # one iteration asked by the author, no choice by distance (REV-2, D-060)
]


class _Contract(BaseModel):
    model_config = ConfigDict(extra="forbid", allow_inf_nan=False)


class Constraints(_Contract):
    """EVA-4: the technical constraints of the mix (D-053)."""

    true_peak_ok: bool = Field(description="True peak at or below the ceiling of the mix.")
    low_corr_ok: bool = Field(description="L/R correlation below 120 Hz above 0 (or no lows).")
    dc_ok: bool = Field(description="DC offset of every channel at most 0.1% of full scale.")

    @property
    def all_ok(self) -> bool:
        return self.true_peak_ok and self.low_corr_ok and self.dc_ok


class Evaluation(_Contract):
    """Content of evaluation.json (PRD §6.4, D-049)."""

    iteration: int | None = Field(ge=0, description="Iteration evaluated; null for the raw sum.")
    spectral_distance_db: float | None = Field(
        ge=0.0, description="Mean absolute band difference from the target, dB; null without one."
    )
    band_delta_db: Bands | None = Field(
        description="Target minus mix per band after aligning the mean level: positive = missing."
    )
    width_delta_db: Db | None = Field(
        description="Target minus mix side/mid ratio, dB: positive = the mix is narrower."
    )
    dyn_delta_lu: Annotated[float, Field(ge=-120.0, le=120.0)] | None = Field(
        description="Target minus mix loudness range, LU: positive = the mix is less dynamic."
    )
    masking_top: list[CollisionEntry] = Field(
        max_length=10, description="Residual collisions between the processed stems."
    )
    constraints: Constraints
    is_best: bool = Field(description="Best iteration at the time it was evaluated (D-049).")
    stop_reason: StopReason | None = Field(description="Why the loop stopped after this one.")
    mix: MixEntry = Field(description="Measures of the evaluated mix (ANA-7, ANA-10).")


class Best(_Contract):
    """Content of best.json: the best iteration of the run (EVA-6, D-049)."""

    iteration: int | None = Field(ge=0, description="Best iteration; null if none is valid.")
    plan: str | None = Field(description="Path of its plan, relative to the run folder.")
    spectral_distance_db: float | None = Field(ge=0.0, description="Its spectral distance, dB.")
    stop_reason: StopReason | None = Field(description="Why the loop stopped; null while running.")


def dumps_evaluation(evaluation: Evaluation) -> str:
    return dumps_document(evaluation.model_dump(mode="json"))


def dumps_best(best: Best) -> str:
    return dumps_document(best.model_dump(mode="json"))


# --- evaluation (EVA-2 → EVA-4) ----------------------------------------------------------------


def spectral_distance(
    mix_bands: Sequence[float], target_bands: Sequence[float]
) -> tuple[float, list[float]]:
    """EVA-2 (D-050): mean absolute band difference after aligning the mean level to the target.

    `offset = mean(target) - mean(mix)` (arithmetic mean of the 8 levels in dB), `delta_i =
    target_i - (mix_i + offset)`, distance = mean |delta_i|. Levels are first limited to ±120 dB,
    so an empty band never gives an infinite distance. Returns `(distance, deltas)`.
    """
    if len(mix_bands) != len(target_bands):
        raise ValueError("mix and target need the same number of bands")
    mix = [limited_db(v) for v in mix_bands]
    target = [limited_db(v) for v in target_bands]
    offset = sum(target) / len(target) - sum(mix) / len(mix)
    delta = [t - (m + offset) for m, t in zip(mix, target, strict=True)]
    return sum(abs(d) for d in delta) / len(delta), delta


def _difference(target: float | None, mix: float | None) -> float | None:
    """EVA-3 (D-050): target minus mix, positive = the mix lacks it; None if either is missing."""
    if target is None or mix is None:
        return None
    return rounded(limited_db(limited_db(target) - limited_db(mix)))  # contract: ±120


def check_constraints(measures: MixMeasures, *, dc_pct: float, ceiling_db: float) -> Constraints:
    """EVA-4 (D-053), on full-precision measures of the trimmed mix."""
    return Constraints(
        true_peak_ok=measures.true_peak_db <= ceiling_db,
        low_corr_ok=measures.low_corr is None or measures.low_corr > 0.0,
        dc_ok=dc_pct <= DC_LIMIT_PCT,
    )


def evaluate_measures(
    measures: MixMeasures,
    target: Target | None,
    *,
    dc_pct: float,
    ceiling_db: float,
    iteration: int | None = None,
    collisions: Sequence[Collision] = (),
) -> Evaluation:
    """Assemble the evaluation of a mix from its measures (EVA-1 → EVA-4).

    `is_best` and `stop_reason` are left to the loop (EVA-5, EVA-6); `collisions` are the
    residual collisions of the processed stems, strongest first (EVA-3, D-054).
    """
    distance: float | None = None
    deltas: list[float] | None = None
    if target is not None and target.bands_db is not None and measures.bands_db is not None:
        full, raw = spectral_distance(measures.bands_db, target.bands_db)
        distance, deltas = rounded(full), [rounded(limited_db(d)) for d in raw]
    mix_side_mid = measures.stereo.side_mid_db if measures.stereo else None
    return Evaluation(
        iteration=iteration,
        spectral_distance_db=distance,
        band_delta_db=deltas,
        width_delta_db=None if target is None else _difference(target.side_mid_db, mix_side_mid),
        dyn_delta_lu=None
        if target is None
        else _difference(target.dyn_range_lu, measures.dyn_range_lu),
        masking_top=collision_entries(list(collisions)[:10]),
        constraints=check_constraints(measures, dc_pct=dc_pct, ceiling_db=ceiling_db),
        is_best=False,
        stop_reason=None,
        mix=mix_entry(measures),
    )


def evaluate_mix(
    audio: np.ndarray,
    sample_rate: int,
    track_spectra: Mapping[str, Spectrum],
    target: Target | None,
    *,
    ceiling_db: float,
    iteration: int | None = None,
    measures: MixMeasures | None = None,
) -> Evaluation:
    """EVA-1: re-analyse a trimmed mix (ANA-7, ANA-10) and the collisions of its stems (ANA-9).

    `track_spectra` are the spectra of the stems as they reach the bus, in project order: the
    processed stems for a rendered mix, the original stems for the raw sum (D-054). `measures`
    avoids measuring the mix again when already done (the raw sum is measured by the analysis).
    """
    measures = measure_mix(audio, sample_rate) if measures is None else measures
    return evaluate_measures(
        measures,
        target,
        dc_pct=dc_offset_pct(audio),
        ceiling_db=ceiling_db,
        iteration=iteration,
        collisions=masking_top(dict(track_spectra)),
    )
