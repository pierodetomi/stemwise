"""Decision inputs for tests: a synthetic `analysis.json` built without audio."""

from __future__ import annotations

from typing import Any

from stemwise.analysis.model import Analysis, dumps_analysis
from stemwise.decision.base import DecisionInput, HistoryEntry, StemInfo
from stemwise.decision.schema import MixPlan
from stemwise.project import Brief

ALL_ROLES = (
    "kick",
    "snare",
    "toms",
    "overheads",
    "drums",
    "percussion",
    "bass",
    "electric_guitar",
    "acoustic_guitar",
    "keys",
    "synth",
    "pad",
    "lead_vocal",
    "backing_vocal",
    "fx",
    "other",
)

BANDS = [-30.0, -20.0, -12.0, -8.0, -5.0, -10.0, -15.0, -22.0]


def _mix_entry() -> dict[str, Any]:
    return {
        "lufs": -18.0,
        "peak_db": -3.5,
        "true_peak_db": -3.0,
        "rms_db": -17.0,
        "crest_db": 13.5,
        "dyn_range_lu": 7.0,
        "bands_db": list(BANDS),
        "stereo": {"correlation": 0.8, "side_mid_db": -11.0},
        "low_corr": 0.95,
    }


def _stem_entry(role: str, channels: int) -> dict[str, Any]:
    return {
        "role": role,
        "channels": channels,
        "lufs": -21.0,
        "peak_db": -6.0,
        "true_peak_db": -5.8,
        "rms_db": -20.0,
        "crest_db": 14.0,
        "dyn_range_lu": 8.0,
        "active_pct": 70.0,
        "bands_db": list(BANDS),
        "centroid_hz": 1500.0,
        "onsets_per_s": 2.0,
        "stereo": None if channels == 1 else {"correlation": 0.7, "side_mid_db": -10.0},
        "hygiene": {"dc_offset": 0.0, "clipped_samples": 0, "noise_floor_db": -80.0},
    }


def fake_analysis(
    stems: list[tuple[str, str, int]],
    *,
    sample_rate: int = 48_000,
    collisions: list[tuple[str, str, str, float]] = (),
    target: bool = True,
) -> Analysis:
    """`stems` are `(id, role, channels)`; `collisions` are `(a, b, band label, score)`."""
    n = len(stems)
    doc = {
        "version": 1,
        "project": {"sample_rate": sample_rate, "duration_s": 10.0},
        "stems": {sid: _stem_entry(role, ch) for sid, role, ch in stems},
        "mix": _mix_entry(),
        "band_share_pct": {sid: [round(100.0 / n, 1)] * 8 for sid, _, _ in stems},
        "masking_top": [{"a": a, "b": b, "band": band, "score": s} for a, b, band, s in collisions],
        "references": [{"id": "ref1", **_mix_entry()}] if target else [],
        "target": {
            "n_references": 1,
            "bands_db": list(BANDS),
            "side_mid_db": -9.5,
            "dyn_range_lu": 6.8,
        }
        if target
        else None,
    }
    return Analysis.model_validate(doc)


DEFAULT_STEMS: list[tuple[str, str, int]] = [
    ("kick", "kick", 1),
    ("bass", "bass", 1),
    ("gtr", "electric_guitar", 2),
    ("lead_vox", "lead_vocal", 1),
]


def make_input(
    stems: list[tuple[str, str, int]] | None = None,
    *,
    sample_rate: int = 48_000,
    collisions: list[tuple[str, str, str, float]] = (),
    notes: dict[str, str] | None = None,
    brief: Brief | None = None,
    iteration: int = 0,
    history: tuple[HistoryEntry, ...] = (),
    request: str | None = None,
    previous_plan: MixPlan | None = None,
) -> DecisionInput:
    stems = DEFAULT_STEMS if stems is None else stems
    analysis = fake_analysis(stems, sample_rate=sample_rate, collisions=collisions)
    notes = notes or {}
    return DecisionInput(
        brief=brief or Brief(genre="indie rock", mood="warm, intimate", notes="Vocal in front."),
        stems=tuple(StemInfo(sid, role, notes.get(sid)) for sid, role, _ in stems),
        analysis=analysis,
        analysis_json=dumps_analysis(analysis),
        iteration=iteration,
        history=history,
        request=request,
        previous_plan=previous_plan,
    )
