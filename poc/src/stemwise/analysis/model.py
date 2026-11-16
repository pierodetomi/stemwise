"""Contract of `analysis.json` (PRD §6.2, ANA-13, D-018) and its conversion and serialization.

`Analysis` is the contract exported to `schemas/analysis.schema.json`. Measures are computed at
full precision; this module is the only place where they are limited and rounded:
- levels in dB (and LU) are limited to [-120, +120], so -inf/+inf never reach the file;
- every real number is rounded to one decimal (ANA-13), except the dimensionless quantities in
  [-1, 1] (correlations, masking scores), rounded to two (D-021); integers stay integers.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from typing import Annotated, Any, Literal

from pydantic import BaseModel, ConfigDict, Field

from stemwise.analysis.masking import Collision
from stemwise.analysis.mix import MixMeasures
from stemwise.analysis.reference import Target
from stemwise.analysis.spectral import BAND_LABELS, N_BANDS
from stemwise.analysis.stem import StemMeasures, StereoMeasures
from stemwise.project import Role, StemId

ANALYSIS_VERSION = 1
DB_LIMIT = 120.0
DECIMALS = 1
RATIO_DECIMALS = 2  # correlation, low_corr, masking score (D-021)

Db = Annotated[float, Field(ge=-DB_LIMIT, le=DB_LIMIT, description="Level in dB, limited to ±120.")]
Lu = Annotated[float, Field(ge=0.0, le=DB_LIMIT, description="Loudness range in LU.")]
Pct = Annotated[float, Field(ge=0.0, le=100.0)]
Corr = Annotated[float, Field(ge=-1.0, le=1.0)]
Bands = Annotated[list[Db], Field(min_length=N_BANDS, max_length=N_BANDS)]
BandLabel = Literal["20-60", "60-120", "120-250", "250-500", "500-2k", "2k-4k", "4k-8k", "8k-20k"]


class _Contract(BaseModel):
    model_config = ConfigDict(extra="forbid", allow_inf_nan=False)


class ProjectInfo(_Contract):
    sample_rate: int = Field(gt=0, description="Sample rate of the project in Hz.")
    duration_s: float = Field(ge=0.0, description="Duration of the (padded) stems in seconds.")


class Stereo(_Contract):
    correlation: Corr | None = Field(description="L/R correlation, 20 Hz-20 kHz, active frames.")
    side_mid_db: Db | None = Field(description="Side/mid power ratio in dB.")


class HygieneEntry(_Contract):
    dc_offset: float = Field(ge=0.0, description="DC offset in percent of full scale.")
    clipped_samples: int = Field(ge=0, description="Samples at or above 1 - 2^-15 (all channels).")
    noise_floor_db: Db | None = Field(description="Median level of the inactive frames, dBFS.")


class StemEntry(_Contract):
    role: Role
    channels: Literal[1, 2]
    lufs: Db | None = Field(description="Integrated loudness, LUFS (BS.1770).")
    peak_db: Db = Field(description="Sample peak, dBFS.")
    true_peak_db: Db = Field(description="True peak with 4x oversampling, dBTP.")
    rms_db: Db | None = Field(description="RMS over the active frames, dBFS.")
    crest_db: Db | None = Field(description="Sample peak minus RMS, dB.")
    dyn_range_lu: Lu | None = Field(description="Loudness range (P95 - P10 of short-term), LU.")
    active_pct: Pct = Field(description="Percentage of frames above -50 dBFS.")
    bands_db: Bands | None = Field(description="Energy of the 8 bands relative to their sum, dB.")
    centroid_hz: float | None = Field(ge=0.0, description="Spectral centroid of active frames.")
    onsets_per_s: float = Field(ge=0.0, description="Onsets per second of active time.")
    stereo: Stereo | None = Field(description="Stereo measures; null for a mono stem.")
    hygiene: HygieneEntry


class MixEntry(_Contract):
    lufs: Db | None
    peak_db: Db
    true_peak_db: Db
    rms_db: Db | None
    crest_db: Db | None
    dyn_range_lu: Lu | None
    bands_db: Bands | None
    stereo: Stereo | None
    low_corr: Corr | None = Field(description="L/R correlation over 20-120 Hz.")


class CollisionEntry(_Contract):
    a: StemId
    b: StemId
    band: BandLabel
    score: float = Field(ge=0.0, le=1.0, description="Masking score in [0, 1].")


class ReferenceEntry(MixEntry):
    id: str = Field(
        pattern=r"^ref[1-3]$",
        description="Neutral label: refN is the N-th reference of project.yaml (no file names).",
    )


class TargetEntry(_Contract):
    n_references: int = Field(ge=1)
    bands_db: Bands | None
    side_mid_db: Db | None
    dyn_range_lu: Lu | None


class Analysis(_Contract):
    """Content of analysis.json: measures of stems, mix and references (PRD §6.2)."""

    version: Literal[1] = ANALYSIS_VERSION
    project: ProjectInfo
    stems: dict[StemId, StemEntry]
    mix: MixEntry
    band_share_pct: dict[
        StemId, Annotated[list[Pct], Field(min_length=N_BANDS, max_length=N_BANDS)]
    ]
    masking_top: list[CollisionEntry] = Field(max_length=10)
    references: list[ReferenceEntry] = Field(max_length=3)
    target: TargetEntry | None = Field(description="Mean profile of the references; null if none.")


# --- conversion: full-precision measures -> contract ---------------------------------------


def _num(value: float) -> float:
    """Round to DECIMALS; never returns -0.0."""
    return round(value, DECIMALS) + 0.0


def _ratio(value: float) -> float:
    """Round a dimensionless quantity in [-1, 1] to RATIO_DECIMALS; never returns -0.0."""
    return round(value, RATIO_DECIMALS) + 0.0


def _db(value: float) -> float:
    return _num(min(DB_LIMIT, max(-DB_LIMIT, value)))


def _opt(value: float | None, fn=_num) -> float | None:
    return None if value is None else fn(value)


def _bands(values: list[float] | None) -> list[float] | None:
    return None if values is None else [_db(v) for v in values]


def _stereo(m: StereoMeasures | None) -> Stereo | None:
    if m is None:
        return None
    return Stereo(correlation=_opt(m.correlation, _ratio), side_mid_db=_opt(m.side_mid_db, _db))


def _mix_fields(m: MixMeasures) -> dict[str, Any]:
    return {
        "lufs": _opt(m.lufs, _db),
        "peak_db": _db(m.peak_db),
        "true_peak_db": _db(m.true_peak_db),
        "rms_db": _opt(m.rms_db, _db),
        "crest_db": _opt(m.crest_db, _db),
        "dyn_range_lu": _opt(m.dyn_range_lu, _db),
        "bands_db": _bands(m.bands_db),
        "stereo": _stereo(m.stereo),
        "low_corr": _opt(m.low_corr, _ratio),
    }


def mix_entry(m: MixMeasures) -> MixEntry:
    return MixEntry(**_mix_fields(m))


def stem_entry(role: str, m: StemMeasures) -> StemEntry:
    return StemEntry(
        role=role,
        channels=m.channels,
        lufs=_opt(m.lufs, _db),
        peak_db=_db(m.peak_db),
        true_peak_db=_db(m.true_peak_db),
        rms_db=_opt(m.rms_db, _db),
        crest_db=_opt(m.crest_db, _db),
        dyn_range_lu=_opt(m.dyn_range_lu, _db),
        active_pct=_num(m.active_pct),
        bands_db=_bands(m.bands_db),
        centroid_hz=_opt(m.centroid_hz),
        onsets_per_s=_num(m.onsets_per_s),
        stereo=_stereo(m.stereo),
        hygiene=HygieneEntry(
            dc_offset=_num(m.hygiene.dc_offset_pct),
            clipped_samples=m.hygiene.clipped_samples,
            noise_floor_db=_opt(m.hygiene.noise_floor_db, _db),
        ),
    )


def collision_entries(collisions: list[Collision]) -> list[CollisionEntry]:
    return [
        CollisionEntry(a=c.a, b=c.b, band=BAND_LABELS[c.band], score=_ratio(c.score))
        for c in collisions
    ]


def rounded(value: float) -> float:
    """A real number as written in the contracts: one decimal (ANA-13), never -0.0."""
    return _num(value)


def limited_db(value: float) -> float:
    """A level in dB limited to ±DB_LIMIT (D-018), at full precision."""
    return min(DB_LIMIT, max(-DB_LIMIT, value))


@dataclass(frozen=True)
class StemInput:
    role: str
    measures: StemMeasures


def build_analysis(
    *,
    sample_rate: int,
    duration_s: float,
    stems: dict[str, StemInput],
    mix: MixMeasures,
    band_share: dict[str, list[float]],
    collisions: list[Collision],
    references: list[MixMeasures],
    target: Target | None,
) -> Analysis:
    """Assemble the contract from full-precision measures, limiting and rounding them."""
    return Analysis(
        project=ProjectInfo(sample_rate=sample_rate, duration_s=_num(duration_s)),
        stems={sid: stem_entry(s.role, s.measures) for sid, s in stems.items()},
        mix=mix_entry(mix),
        band_share_pct={sid: [_num(v) for v in shares] for sid, shares in band_share.items()},
        masking_top=collision_entries(collisions),
        references=[
            ReferenceEntry(id=f"ref{i}", **_mix_fields(m)) for i, m in enumerate(references, 1)
        ],
        target=target_entry(target),
    )


def target_entry(target: Target | None) -> TargetEntry | None:
    """The `target` block of analysis.json: the profile limited and rounded (ANA-12)."""
    if target is None:
        return None
    return TargetEntry(
        n_references=target.n_references,
        bands_db=_bands(target.bands_db),
        side_mid_db=_opt(target.side_mid_db, _db),
        dyn_range_lu=_opt(target.dyn_range_lu, _db),
    )


# --- serialization ---------------------------------------------------------------------------


def _dump(value: Any, indent: int) -> str:
    """JSON with objects indented by two spaces and lists of scalars on a single line."""
    pad, inner = "  " * indent, "  " * (indent + 1)
    if isinstance(value, dict):
        if not value:
            return "{}"
        items = [f"{inner}{json.dumps(k)}: {_dump(v, indent + 1)}" for k, v in value.items()]
        return "{\n" + ",\n".join(items) + "\n" + pad + "}"
    if isinstance(value, list):
        if all(not isinstance(v, dict | list) for v in value):
            return "[" + ", ".join(_dump(v, indent) for v in value) + "]"
        items = [f"{inner}{_dump(v, indent + 1)}" for v in value]
        return "[\n" + ",\n".join(items) + "\n" + pad + "]"
    if isinstance(value, float) and not math.isfinite(value):
        raise ValueError(f"non-finite number in analysis.json: {value}")
    return json.dumps(value, ensure_ascii=False)


def dumps_document(doc: Any) -> str:
    """JSON layout of the contracts: objects indented, lists of scalars on one line, newline."""
    return _dump(doc, 0) + "\n"


def dumps_analysis(analysis: Analysis) -> str:
    """Serialize `analysis` (keys in contract order) with a trailing newline."""
    return dumps_document(analysis.model_dump(mode="json"))


def estimated_tokens(text: str) -> float:
    """ANA-13: token estimate = characters / 4."""
    return len(text) / 4
