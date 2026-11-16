"""Revision in words (REV-1 → REV-3, D-059 → D-066).

A revision is a new run that starts from the best plan of an earlier run and makes one decision
with the author's request as top priority. This module holds the pure parts: the contract of
`revision.json` (D-059), the choice of the source run and its consistency with the project
(D-060, D-061) and the diff between two plans (REV-3, D-064). Files are written by `runs.py`.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Annotated, Any

from pydantic import BaseModel, ConfigDict, Field, StringConstraints, ValidationError

from stemwise.analysis.model import Analysis, dumps_document, limited_db, rounded, target_entry
from stemwise.analysis.reference import Target
from stemwise.analysis.stem import sample_peak_db
from stemwise.audio_io import Stems
from stemwise.decision.schema import Compressor, EqBand, MixPlan, Reverb, StemPlan, load_plan
from stemwise.errors import ReportError, StemwiseError
from stemwise.evaluate import Best, Evaluation

MAX_REQUEST_CHARS = 1000  # D-066: keeps the request within a sensible token budget

Request = Annotated[
    str,
    StringConstraints(
        strip_whitespace=True, min_length=1, max_length=MAX_REQUEST_CHARS, pattern=r"\S"
    ),
]  # `pattern` tells a schema-only validator that blank text is refused


class Revision(BaseModel):
    """Content of revision.json: where a revision starts from and what the author asked (D-059)."""

    model_config = ConfigDict(extra="forbid", allow_inf_nan=False)

    source_run: str = Field(
        min_length=1,
        description="Run the revision starts from: its folder name in runs/, or a path.",
    )
    source_iteration: int = Field(ge=0, description="Iteration of the source run's best plan.")
    source_plan: str = Field(
        min_length=1, description="Path of the starting plan, relative to the source run."
    )
    request: Request = Field(description="The author's request, in natural language.")


def dumps_revision(revision: Revision) -> str:
    return dumps_document(revision.model_dump(mode="json"))


class RevisionError(StemwiseError):
    """A revision cannot start: no valid plan to start from, or the project has changed."""


# --- source run (D-060, D-061) ------------------------------------------------------------------


@dataclass(frozen=True)
class Source:
    """The run a revision starts from: its best plan and the files the revision copies."""

    run_dir: Path
    label: str  # `source_run` of revision.json: the folder name for a run of the project
    iteration: int
    plan_path: str  # relative to the source run, as in its best.json
    plan: MixPlan
    evaluation: Evaluation
    evaluation_text: str  # exact text of its evaluation.json, copied as source_evaluation.json
    analysis: Analysis
    analysis_text: str
    baseline_evaluation: Evaluation
    baseline_evaluation_text: str

    @property
    def baseline_file(self) -> Path:
        return self.run_dir / "baseline.wav"


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ReportError(f"cannot read {path}: {exc}") from exc


def _validated(model: type[BaseModel], path: Path) -> tuple[Any, str]:
    text = _read_text(path)
    try:
        return model.model_validate_json(text), text
    except ValidationError as exc:
        raise ReportError(f"cannot read {path}: not a valid {path.name}:\n{exc}") from exc


def _label(project_root: Path, run_dir: Path) -> str:
    runs = (Path(project_root) / "runs").resolve()
    return run_dir.name if run_dir.resolve().parent == runs else str(run_dir)


def load_source(project_root: Path, run_dir: Path) -> Source:
    """D-060: the plan named by `best.json` of `run_dir`, with its evaluation and the analysis."""
    run_dir = Path(run_dir)
    best_file = run_dir / "best.json"
    if not best_file.is_file():
        raise ReportError(
            f"{run_dir} has no best.json: it is not a mix run with evaluations (made before M4, "
            "or interrupted before the first evaluation)"
        )
    best, _ = _validated(Best, best_file)
    if best.iteration is None or best.plan is None:
        raise RevisionError(
            f"run {run_dir.name} has no plan within the technical constraints to start from: "
            "choose an earlier run with --run"
        )
    plan_file = run_dir / best.plan
    evaluation, evaluation_text = _validated(Evaluation, plan_file.parent / "evaluation.json")
    analysis, analysis_text = _validated(Analysis, run_dir / "analysis.json")
    baseline, baseline_text = _validated(Evaluation, run_dir / "baseline_evaluation.json")
    if not (run_dir / "baseline.wav").is_file():
        raise ReportError(f"{run_dir} has no baseline.wav")
    return Source(
        run_dir=run_dir,
        label=_label(project_root, run_dir),
        iteration=best.iteration,
        plan_path=best.plan,
        plan=load_plan(plan_file),
        evaluation=evaluation,
        evaluation_text=evaluation_text,
        analysis=analysis,
        analysis_text=analysis_text,
        baseline_evaluation=baseline,
        baseline_evaluation_text=baseline_text,
    )


def _differences(
    analysis: Analysis, stems: Stems, roles: dict[str, str], target: Target | None
) -> list[str]:
    out: list[str] = []
    if analysis.project.sample_rate != stems.sample_rate:
        out.append(f"sample rate {analysis.project.sample_rate} -> {stems.sample_rate} Hz")
    duration = rounded(stems.frames / stems.sample_rate)
    if analysis.project.duration_s != duration:
        out.append(f"duration {analysis.project.duration_s:g} -> {duration:g} s")
    before_ids, after_ids = list(analysis.stems), list(stems.audio)
    if before_ids != after_ids:
        out.append(f"stems {', '.join(before_ids)} -> {', '.join(after_ids)}")
    for sid, audio in stems.audio.items():
        entry = analysis.stems.get(sid)
        if entry is None:
            continue
        if entry.role != roles[sid]:
            out.append(f"{sid}: role {entry.role} -> {roles[sid]}")
        if entry.channels != audio.shape[1]:
            out.append(f"{sid}: channels {entry.channels} -> {audio.shape[1]}")
        peak = rounded(limited_db(sample_peak_db(audio)))  # as in analysis.json (D-018)
        if entry.peak_db != peak:
            out.append(f"{sid}: peak {entry.peak_db:g} -> {peak:g} dBFS")
    before_n = analysis.target.n_references if analysis.target is not None else 0
    after_n = target.n_references if target is not None else 0
    if before_n != after_n:
        out.append(f"references {before_n} -> {after_n}")
    elif analysis.target != target_entry(target):
        out.append("target profile of the references")
    return out


def check_consistency(
    source: Source, stems: Stems, roles: dict[str, str], target: Target | None
) -> None:
    """D-061: the project must still be the one the source analysis describes.

    Checked: sample rate, duration, stem ids and order, and per stem the role, the channels and
    the sample peak; the target profile measured again from the references must round to the
    saved one. Changed stems or references make the analysis (and the plan) stale.
    """
    differences = _differences(source.analysis, stems, roles, target)
    if differences:
        raise RevisionError(
            f"the project no longer matches run {source.label} ("
            + "; ".join(differences)
            + "): run `stemwise mix` again"
        )


# --- plan diff (REV-3, D-064) -------------------------------------------------------------------

Value = float | str | None
REVERB_TARGET = "(reverb)"  # not a valid stem id (D-010): `reverb` is one
_SCALARS = ("gain_db", "pan", "hpf_hz", "lpf_hz")
_COMPRESSOR_FIELDS = ("threshold_db", "ratio", "attack_ms", "release_ms", "makeup_db")
_EQ_FIELDS = ("freq_hz", "gain_db", "q")
_REVERB_FIELDS = ("room_size", "damping", "return_hpf_hz", "return_gain_db")


@dataclass(frozen=True)
class Change:
    """One parameter that differs between two plans: `target` is a stem id or `REVERB_TARGET`.

    `parameter` is a field name, `eq[n].<field>` (bands numbered from 1, matched by position as
    in DEC-5), `eq[n]` for a band added, removed or of another type, `compressor.<field>`, or a
    whole block (`compressor`, `reverb`) switched on or off; whole blocks and bands are described
    as text, `None` meaning off or absent.
    """

    target: str
    parameter: str
    before: Value
    after: Value


def band_text(band: EqBand) -> str:
    return f"{band.type} {band.freq_hz:g} Hz {band.gain_db:+.1f} dB Q {band.q:g}"


def compressor_text(c: Compressor) -> str:
    return (
        f"{c.threshold_db:g} dB, {c.ratio:g}:1, {c.attack_ms:g}/{c.release_ms:g} ms, "
        f"makeup {c.makeup_db:g} dB"
    )


def reverb_text(r: Reverb) -> str:
    return (
        f"room {r.room_size:g}, damping {r.damping:g}, return high-pass {r.return_hpf_hz:g} Hz, "
        f"{r.return_gain_db:g} dB"
    )


def _fields(target: str, prefix: str, before: object, after: object, names) -> list[Change]:
    out = []
    for name in names:
        a, b = getattr(before, name), getattr(after, name)
        if a != b:
            out.append(Change(target, prefix + name, a, b))
    return out


def _eq_diff(sid: str, before: list[EqBand], after: list[EqBand]) -> list[Change]:
    out: list[Change] = []
    for i in range(max(len(before), len(after))):
        a = before[i] if i < len(before) else None
        b = after[i] if i < len(after) else None
        name = f"eq[{i + 1}]"
        if a is not None and b is not None and a.type == b.type:
            out += _fields(sid, name + ".", a, b, _EQ_FIELDS)
        elif a != b:
            out.append(
                Change(
                    sid,
                    name,
                    None if a is None else band_text(a),
                    None if b is None else band_text(b),
                )
            )
    return out


def _stem_diff(sid: str, a: StemPlan, b: StemPlan) -> list[Change]:
    out = _fields(sid, "", a, b, _SCALARS)
    out += _eq_diff(sid, a.eq_bands, b.eq_bands)
    if a.compressor is not None and b.compressor is not None:
        out += _fields(sid, "compressor.", a.compressor, b.compressor, _COMPRESSOR_FIELDS)
    elif a.compressor != b.compressor:
        out.append(
            Change(
                sid,
                "compressor",
                None if a.compressor is None else compressor_text(a.compressor),
                None if b.compressor is None else compressor_text(b.compressor),
            )
        )
    out += _fields(sid, "", a, b, ("reverb_send_db",))
    return out


def plan_diff(before: MixPlan, after: MixPlan) -> list[Change]:
    """REV-3 (D-064): every parameter that differs, stems in the order of `after`, then reverb.

    Values are compared exactly as written in the plans; `rationale`, `summary` and `iteration`
    are not parameters. Both plans must have the same stems (they fit the same project).
    """
    if set(before.stems) != set(after.stems):
        raise ValueError("the two plans must have the same stems")
    out: list[Change] = []
    for sid, sp in after.stems.items():
        out += _stem_diff(sid, before.stems[sid], sp)
    a, b = before.reverb, after.reverb
    if a is not None and b is not None:
        out += _fields(REVERB_TARGET, "", a, b, _REVERB_FIELDS)
    elif a != b:
        out.append(
            Change(
                REVERB_TARGET,
                "reverb",
                None if a is None else reverb_text(a),
                None if b is None else reverb_text(b),
            )
        )
    return out


def unchanged_stems(before: MixPlan, after: MixPlan) -> list[str]:
    """Stems of `after` (in its order) with no changed parameter."""
    changed = {c.target for c in plan_diff(before, after)}
    return [sid for sid in after.stems if sid not in changed]
