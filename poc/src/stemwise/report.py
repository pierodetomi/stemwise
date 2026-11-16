"""`report.md` of a run (REP-1, D-056), built from the files of the run folder only.

`load_report_data` reads `analysis.json`, `baseline_evaluation.json`, `best.json`, every
`iter_NN/` (plan, evaluation, decider logs) and, for a revision run (D-066), `revision.json`,
`source_plan.json` and `source_evaluation.json`; `render_report` is a pure function of that
data, so `stemwise report` regenerates exactly the file written by `mix` or `revise`. The fixed
text is in English (engine output, like the CLI); summaries and rationales are written as the
decider wrote them.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from pydantic import BaseModel, ValidationError

from stemwise.analysis.model import Analysis, CollisionEntry
from stemwise.decision.schema import MixPlan, StemPlan, load_plan
from stemwise.errors import ReportError
from stemwise.evaluate import Best, Evaluation
from stemwise.revise import REVERB_TARGET, Revision, plan_diff, unchanged_stems

REPORT_FILE = "report.md"
TOP_COLLISIONS = 5
_ITER_DIR = re.compile(r"^iter_(\d{2})$")
_RUN_NAME = re.compile(r"^(\d{4}-\d{2}-\d{2}T\d{2}-\d{2}-\d{2})(?:-(\d+))?$")

STOP_TEXT = {
    "below_threshold": "the spectral distance fell below 1.5 dB",
    "no_improvement": "the distance improved by less than 0.2 dB on the previous iteration",
    "max_iterations": "the maximum number of iterations was reached",
    "no_target": "no reference: stopped at the first iteration within the constraints",
    "decision_failed": "the decision of the next iteration failed",
    "revision": "one revision asked by the author (no choice by distance)",
}


@dataclass(frozen=True)
class IterationData:
    iteration: int
    plan: MixPlan | None  # None for a decision that failed
    evaluation: Evaluation | None
    request_log: dict[str, Any] | None
    response_log: dict[str, Any] | None


@dataclass(frozen=True)
class RevisionData:
    """What a revision run adds (D-059): where it starts from, the request, the starting plan."""

    revision: Revision
    source_plan: MixPlan
    source_evaluation: Evaluation


@dataclass(frozen=True)
class ReportData:
    run_name: str
    analysis: Analysis
    baseline: Evaluation
    best: Best
    iterations: list[IterationData]
    revision: RevisionData | None = None


def _read_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        raise ReportError(f"cannot read {path}: {exc}") from exc


def _validated(model: type[BaseModel], path: Path) -> Any:
    try:
        return model.model_validate(_read_json(path))
    except ValidationError as exc:
        raise ReportError(f"cannot read {path}: not a valid {path.name}:\n{exc}") from exc


def _optional_json(path: Path) -> Any:
    return _read_json(path) if path.is_file() else None


def load_report_data(run_dir: Path) -> ReportData:
    run_dir = Path(run_dir)
    best_file = run_dir / "best.json"
    if not best_file.is_file():
        raise ReportError(
            f"{run_dir} has no best.json: it is not a mix run with evaluations (made before M4, "
            "or interrupted before the first evaluation)"
        )
    iterations = []
    for path in sorted(p for p in run_dir.iterdir() if p.is_dir() and _ITER_DIR.match(p.name)):
        plan_file, evaluation_file = path / "plan.json", path / "evaluation.json"
        iterations.append(
            IterationData(
                iteration=int(_ITER_DIR.match(path.name).group(1)),
                plan=load_plan(plan_file) if plan_file.is_file() else None,
                evaluation=_validated(Evaluation, evaluation_file)
                if evaluation_file.is_file()
                else None,
                request_log=_optional_json(path / "llm_request.json"),
                response_log=_optional_json(path / "llm_response.json"),
            )
        )
    revision = None
    if (run_dir / "revision.json").is_file():
        revision = RevisionData(
            revision=_validated(Revision, run_dir / "revision.json"),
            source_plan=load_plan(run_dir / "source_plan.json"),
            source_evaluation=_validated(Evaluation, run_dir / "source_evaluation.json"),
        )
    return ReportData(
        run_name=run_dir.name,
        analysis=_validated(Analysis, run_dir / "analysis.json"),
        baseline=_validated(Evaluation, run_dir / "baseline_evaluation.json"),
        best=_validated(Best, best_file),
        iterations=iterations,
        revision=revision,
    )


# --- text --------------------------------------------------------------------------------------


def _cell(text: str) -> str:
    return " ".join(str(text).split()).replace("|", "\\|")


def _num(value: float | None, fmt: str = ".1f") -> str:
    return "—" if value is None else format(value, fmt)


def _signed(value: float | None) -> str:
    return _num(value, "+.1f")


def _constraints(ev: Evaluation) -> str:
    c = ev.constraints
    failed = [
        name
        for name, ok in (
            ("true peak", c.true_peak_ok),
            ("low correlation", c.low_corr_ok),
            ("DC offset", c.dc_ok),
        )
        if not ok
    ]
    return "ok" if not failed else "violated: " + ", ".join(failed)


def _cost(response_log: dict[str, Any] | None) -> tuple[float | None, bool, int, int, int]:
    """(cost, complete, calls, prompt tokens, completion tokens) of one iteration's calls."""
    if not response_log:
        return None, True, 0, 0, 0
    total = response_log.get("total") or {}
    return (
        total.get("cost_usd"),
        bool(total.get("cost_complete", True)) and total.get("cost_usd") is not None,
        int(total.get("attempts", len(response_log.get("attempts", [])))),
        int(total.get("prompt_tokens", 0)),
        int(total.get("completion_tokens", 0)),
    )


def _decider(data: ReportData) -> str:
    for it in data.iterations:
        if it.request_log:
            who = str(it.request_log.get("decider", "?"))
            model = (it.request_log.get("config") or {}).get("model")
            return f"{who} ({model})" if model else who
    return "hand-written plan (`mix --plan`)"


def _cost_line(data: ReportData) -> str:
    logs = [it.response_log for it in data.iterations if it.response_log]
    if not logs:
        return "no model call"
    parts = [_cost(log) for log in logs]
    known = [p[0] for p in parts if p[0] is not None]
    complete = all(p[1] for p in parts)
    calls = sum(p[2] for p in parts)
    tokens_in, tokens_out = sum(p[3] for p in parts), sum(p[4] for p in parts)
    cost = "not reported" if not known else f"{sum(known):.4f} USD"
    if known and not complete:
        cost += " (partial: some calls did not report their cost)"
    return f"{cost}, {calls} call(s), {tokens_in} prompt + {tokens_out} completion tokens"


def _row(label: str, ev: Evaluation, cost: str) -> str:
    cells = [
        label,
        _num(ev.spectral_distance_db),
        _num(ev.mix.lufs),
        _num(ev.mix.true_peak_db),
        _signed(ev.width_delta_db),
        _signed(ev.dyn_delta_lu),
        _constraints(ev),
        cost,
    ]
    return "| " + " | ".join(cells) + " |"


def _iteration_cost(it: IterationData) -> str:
    cost, complete, calls, _, _ = _cost(it.response_log)
    if calls == 0:
        return "—"
    text = "not reported" if cost is None else f"{cost:.4f}"
    return text + ("" if complete or cost is None else " (partial)")


def _eq(sp: StemPlan) -> str:
    if not sp.eq_bands:
        return "—"
    return "; ".join(
        f"{b.type} {b.freq_hz:g} Hz {b.gain_db:+.1f} dB Q {b.q:g}" for b in sp.eq_bands
    )


def _hz(value: float | None) -> str:
    return "—" if value is None else f"{value:g} Hz"


def _compressor(sp: StemPlan) -> str:
    c = sp.compressor
    if c is None:
        return "—"
    return f"{c.threshold_db:g} dB, {c.ratio:g}:1, {c.attack_ms:g}/{c.release_ms:g} ms"


def _collisions(entries: list[CollisionEntry]) -> list[str]:
    entries = [c for c in entries if c.score > 0]  # scores that round to 0.00 say nothing
    if not entries:
        return ["No collision above 0.00."]
    lines = ["| Stems | Band | Score |", "| --- | --- | --- |"]
    lines += [f"| {c.a} – {c.b} | {c.band} Hz | {c.score:.2f} |" for c in entries[:TOP_COLLISIONS]]
    return lines


def render_report(data: ReportData) -> str:
    """REP-1: the text of report.md (of a revision run: `render_revision_report`)."""
    if data.revision is not None:
        return render_revision_report(data)
    target = data.analysis.target
    best = data.best
    best_it = next((it for it in data.iterations if it.iteration == best.iteration), None)
    stop = STOP_TEXT.get(best.stop_reason or "", "the loop did not finish (interrupted)")
    lines = [
        f"# Mix report — run {data.run_name}",
        "",
        f"- Decider: {_decider(data)}",
        f"- Stems: {len(data.analysis.stems)}; references: "
        + (
            f"{target.n_references} (spectral distance from their mean profile)"
            if target is not None
            else "none (only the technical constraints are checked)"
        ),
        f"- Iterations: {sum(1 for it in data.iterations if it.evaluation is not None)}; "
        f"stop: {stop}",
        "- Best: "
        + (
            "none within the technical constraints"
            if best.iteration is None
            else f"iter_{best.iteration:02d} (distance {_num(best.spectral_distance_db)} dB)"
        ),
        f"- Cost of the model calls: {_cost_line(data)}",
        "",
        "## Iterations",
        "",
        "Distance: mean absolute difference from the target over the 8 bands, after aligning the "
        "mean level (lower is better). Δ = target minus mix: a positive width Δ means the mix is "
        "narrower, a positive dynamics Δ that it is less dynamic.",
        "",
        "| | Distance (dB) | LUFS | True peak (dBTP) | Width Δ (dB) | Dynamics Δ (LU) | "
        "Constraints | Cost (USD) |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
        _row("raw sum", data.baseline, "—"),
    ]
    for it in data.iterations:
        name = f"iter_{it.iteration:02d}"
        if it.evaluation is None:
            reason = "decision failed" if it.plan is None else "not evaluated"
            lines.append(f"| {name} | {reason} | | | | | | {_iteration_cost(it)} |")
            continue
        label = f"**{name} (best)**" if it.iteration == best.iteration else name
        lines.append(_row(label, it.evaluation, _iteration_cost(it)))
    lines += ["", "## Summaries", ""]
    for it in data.iterations:
        text = "—" if it.plan is None else _cell(it.plan.summary)
        lines.append(f"- iter_{it.iteration:02d}: {text}")
    warnings = [
        (it.iteration, w)
        for it in data.iterations
        for w in (it.request_log or {}).get("warnings", [])
    ]
    if warnings:
        lines += ["", "## Warnings", ""]
        lines += [f"- iter_{k:02d}: {_cell(w)}" for k, w in warnings]
    if best_it is not None and best_it.plan is not None:
        plan = best_it.plan
        lines += [
            "",
            f"## Best plan (iter_{best_it.iteration:02d})",
            "",
            _cell(plan.summary),
            "",
            "| Stem | Role | Gain | Pan | High-pass | Low-pass | EQ | Compressor | Reverb send | "
            "Rationale |",
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
        ]
        for sid, sp in plan.stems.items():
            role = data.analysis.stems[sid].role if sid in data.analysis.stems else "?"
            send = "—" if sp.reverb_send_db is None else f"{sp.reverb_send_db:g} dB"
            lines.append(
                f"| {sid} | {role} | {sp.gain_db:+.1f} dB | {sp.pan:+.2f} | {_hz(sp.hpf_hz)} | "
                f"{_hz(sp.lpf_hz)} | {_eq(sp)} | {_compressor(sp)} | {send} | "
                f"{_cell(sp.rationale)} |"
            )
        reverb = plan.reverb
        lines += [
            "",
            "Reverb: "
            + (
                "off"
                if reverb is None
                else f"room size {reverb.room_size:g}, damping {reverb.damping:g}, return "
                f"high-pass {reverb.return_hpf_hz:g} Hz, return gain {reverb.return_gain_db:g} dB"
            )
            + ".",
        ]
    lines += ["", "## Main collisions", "", "Raw sum:", "", *_collisions(data.baseline.masking_top)]
    if best_it is not None and best_it.evaluation is not None:
        lines += [
            "",
            f"Left in the best mix (iter_{best_it.iteration:02d}):",
            "",
            *_collisions(best_it.evaluation.masking_top),
        ]
    return "\n".join(lines) + "\n"


# --- revision report (REV-3, D-064, D-066) ------------------------------------------------------

_OFF_PARAMETERS = {"hpf_hz", "lpf_hz", "reverb_send_db", "compressor", "reverb"}


def _diff_value(parameter: str, value: float | str | None) -> str:
    name = parameter.rsplit(".", 1)[-1]
    if value is None:
        return "off" if name in _OFF_PARAMETERS else "—"  # an EQ band that is not there
    if isinstance(value, str):
        return _cell(value)
    if name.endswith("_db"):
        return f"{value:+.1f} dB"
    if name.endswith("_hz"):
        return f"{value:g} Hz"
    if name.endswith("_ms"):
        return f"{value:g} ms"
    if name == "pan":
        return f"{value:+.2f}"
    return f"{value:g}"


def _target_label(target: str) -> str:
    return "shared reverb" if target == REVERB_TARGET else target


def render_revision_report(data: ReportData) -> str:
    """D-066: the report of a revision run, with the diff from the starting plan (REV-3)."""
    rev = data.revision
    assert rev is not None
    target = data.analysis.target
    it = next((i for i in data.iterations if i.iteration == 0), None)
    revised = it.plan if it is not None else None
    evaluation = it.evaluation if it is not None else None
    if data.best.iteration is not None:
        result = "iter_00 is the revised plan (no rollback: judge it by ear)"
    elif evaluation is not None:
        result = (
            "none within the technical constraints: listen to iter_00/mix.wav, then revise "
            f"again from run {rev.revision.source_run} with --run"
        )
    else:
        result = "the revision did not finish (interrupted)"
    lines = [
        f"# Revision report — run {data.run_name}",
        "",
        f'- Request: "{_cell(rev.revision.request)}"',
        f"- Starts from: run {rev.revision.source_run}, iter_{rev.revision.source_iteration:02d} "
        "(its best plan)",
        f"- Decider: {_decider(data)}",
        f"- Stems: {len(data.analysis.stems)}; references: "
        + (
            f"{target.n_references} (spectral distance from their mean profile)"
            if target is not None
            else "none (only the technical constraints are checked)"
        ),
        f"- Result: {result}",
        f"- Cost of the model calls: {_cost_line(data)}",
        "",
        "## Comparison",
        "",
        "Distance: mean absolute difference from the target over the 8 bands, after aligning the "
        "mean level (lower is better); in a revision it is only information. Δ = target minus "
        "mix: a positive width Δ means the mix is narrower, a positive dynamics Δ that it is less "
        "dynamic.",
        "",
        "| | Distance (dB) | LUFS | True peak (dBTP) | Width Δ (dB) | Dynamics Δ (LU) | "
        "Constraints | Cost (USD) |",
        "| --- | --- | --- | --- | --- | --- | --- | --- |",
        _row("raw sum", data.baseline, "—"),
        _row("starting plan", rev.source_evaluation, "—"),
    ]
    if evaluation is not None and it is not None:
        lines.append(_row("**revised plan (iter_00)**", evaluation, _iteration_cost(it)))
    else:
        cost = _iteration_cost(it) if it is not None else "—"
        lines.append(f"| revised plan (iter_00) | not evaluated | | | | | | {cost} |")
    lines += [
        "",
        "## Summaries",
        "",
        f"- starting plan: {_cell(rev.source_plan.summary)}",
        f"- revised plan: {'—' if revised is None else _cell(revised.summary)}",
    ]
    if revised is not None:
        changes = plan_diff(rev.source_plan, revised)
        lines += ["", "## Changes from the starting plan", ""]
        if changes:
            lines += ["| Stem | Parameter | Before | After |", "| --- | --- | --- | --- |"]
            lines += [
                f"| {_target_label(c.target)} | {c.parameter} | "
                f"{_diff_value(c.parameter, c.before)} | {_diff_value(c.parameter, c.after)} |"
                for c in changes
            ]
        else:
            lines.append("No parameter changed.")
        unchanged = unchanged_stems(rev.source_plan, revised)
        lines += ["", f"Unchanged stems: {', '.join(unchanged) if unchanged else 'none'}."]
        changed = [sid for sid in revised.stems if sid not in unchanged]
        if changed:
            lines += [
                "",
                "## Rationales of the changed stems",
                "",
                "| Stem | Role | Rationale |",
                "| --- | --- | --- |",
            ]
            for sid in changed:
                role = data.analysis.stems[sid].role if sid in data.analysis.stems else "?"
                lines.append(f"| {sid} | {role} | {_cell(revised.stems[sid].rationale)} |")
    warnings = (it.request_log or {}).get("warnings", []) if it is not None else []
    if warnings:
        lines += ["", "## Warnings", ""]
        lines += [f"- iter_00: {_cell(w)}" for w in warnings]
    lines += [
        "",
        "## Main collisions",
        "",
        "Left in the starting mix:",
        "",
        *_collisions(rev.source_evaluation.masking_top),
    ]
    if evaluation is not None:
        lines += [
            "",
            "Left in the revised mix (iter_00):",
            "",
            *_collisions(evaluation.masking_top),
        ]
    return "\n".join(lines) + "\n"


def write_report(run_dir: Path) -> Path:
    """Write (or rewrite) `report.md` of a run from its files."""
    text = render_report(load_report_data(run_dir))
    path = Path(run_dir) / REPORT_FILE
    tmp = path.with_name(REPORT_FILE + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    tmp.replace(path)
    return path


def _run_key(path: Path) -> tuple[str, int]:
    match = _RUN_NAME.match(path.name)
    assert match is not None
    return match.group(1), int(match.group(2) or 1)


def latest_run(project_root: Path) -> Path:
    """The most recent run of the project with a `best.json` (D-056)."""
    runs_dir = Path(project_root) / "runs"
    candidates = (
        [
            p
            for p in runs_dir.iterdir()
            if p.is_dir() and _RUN_NAME.match(p.name) and (p / "best.json").is_file()
        ]
        if runs_dir.is_dir()
        else []
    )
    if not candidates:
        raise ReportError(f"no mix run with evaluations in {runs_dir}: run `stemwise mix` first")
    return max(candidates, key=_run_key)


def find_run(project_root: Path, run: Path | None = None) -> Path:
    """The run folder a command works on (PRD §8): the latest one with `best.json` (D-056), or
    `run`, looked up first as a name in `runs/` of the project and then as a path (D-058)."""
    if run is None:
        return latest_run(project_root)
    by_name = Path(project_root) / "runs" / run
    if by_name.is_dir():
        return by_name
    if Path(run).is_dir():
        return Path(run)
    raise ReportError(f"run folder not found: {run}")
