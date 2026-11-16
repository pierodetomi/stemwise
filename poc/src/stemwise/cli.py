"""Command line interface: a thin wrapper over engine functions (PRD §7.1, §8).

Every command takes the project folder (the one holding `project.yaml`). Engine errors
(`StemwiseError`) and file-system errors are printed on stderr without a traceback and exit
with code 1; commands of later milestones exit with code 3.
"""

from __future__ import annotations

import logging
import os
import sys
from enum import StrEnum
from pathlib import Path
from typing import Annotated

import typer

from stemwise import __version__, runs
from stemwise.analysis.model import estimated_tokens
from stemwise.decision.base import Decider
from stemwise.decision.config import REASONING_EFFORTS, LlmConfig, load_llm_config
from stemwise.decision.llm import LlmDecider
from stemwise.decision.mock import MockDecider
from stemwise.errors import StemwiseError
from stemwise.project import load_project
from stemwise.render.mixbus import BusResult
from stemwise.render.render import MixResult
from stemwise.report import find_run, write_report
from stemwise.revise import MAX_REQUEST_CHARS

EXIT_ERROR = 1
EXIT_NOT_IMPLEMENTED = 3  # distinct from 2, used by the CLI parser for usage errors

app = typer.Typer(
    name="stemwise",
    help="AI-assisted mixing PoC: the model only ever sees numeric measurements of the stems.",
    no_args_is_help=True,
    add_completion=False,
    pretty_exceptions_enable=False,
)

ProjectArg = Annotated[
    Path, typer.Argument(help="Project folder containing project.yaml.", show_default=False)
]


class _StderrHandler(logging.Handler):
    """Writes log records to the *current* sys.stderr (so test runners can capture them)."""

    def emit(self, record: logging.LogRecord) -> None:
        sys.stderr.write(f"{record.levelname.lower()}: {record.getMessage()}\n")


def _configure_logging() -> None:
    logger = logging.getLogger("stemwise")
    if not any(isinstance(h, _StderrHandler) for h in logger.handlers):
        logger.addHandler(_StderrHandler())
    logger.setLevel(logging.INFO)


def _fail(message: str, code: int = EXIT_ERROR) -> typer.Exit:
    typer.echo(f"error: {message}", err=True)
    return typer.Exit(code)


def _not_implemented(command: str, milestone: str) -> typer.Exit:
    return _fail(f"'{command}' is not implemented yet ({milestone}).", EXIT_NOT_IMPLEMENTED)


def _version(value: bool) -> None:
    if value:
        typer.echo(f"stemwise {__version__}")
        raise typer.Exit()


@app.callback()
def main(
    version: Annotated[
        bool,
        typer.Option("--version", callback=_version, is_eager=True, help="Show version and exit."),
    ] = False,
) -> None:
    _configure_logging()


@app.command()
def analyze(project: ProjectArg) -> None:
    """Measure stems, mix and references; write analysis.json in a new run."""
    try:
        result = runs.analysis_run(project)
    except (StemwiseError, OSError) as exc:
        raise _fail(str(exc)) from exc
    a = result.analysis
    typer.echo(f"run: {result.run_dir}")
    typer.echo(
        f"analysis: {result.analysis_file} ({len(a.stems)} stems, {len(a.references)} references, "
        f"~{estimated_tokens(result.text):.0f} tokens)"
    )


class DeciderName(StrEnum):
    llm = "llm"
    mock = "mock"


ENV_FILE = Path(".env")  # read from the current folder (D-037)


def _llm_decider(config: LlmConfig) -> Decider:
    """Build the real decider (a function, so tests can inject a fake transport)."""
    return LlmDecider(config)


def _make_decider(
    name: DeciderName, model: str | None, temperature: float | None, effort: str | None
) -> Decider:
    if name is DeciderName.mock:
        return MockDecider()
    # D-044: key and model are checked before any work is done.
    config = load_llm_config(
        os.environ,
        env_file=ENV_FILE,
        model=model,
        temperature=temperature,
        reasoning_effort=effort,
    )
    return _llm_decider(config)


@app.command()
def mix(
    project: ProjectArg,
    baseline_only: Annotated[
        bool, typer.Option("--baseline-only", help="Only write the raw sum (baseline.wav).")
    ] = False,
    plan: Annotated[
        Path | None,
        typer.Option(
            "--plan",
            help="Render this hand-written mix_plan.json instead of asking a decider.",
            show_default=False,
        ),
    ] = None,
    decider: Annotated[
        DeciderName | None,
        typer.Option(
            "--decider",
            help="Who decides the plan: the real model (llm, default) or the mock.",
            show_default=False,
        ),
    ] = None,
    model: Annotated[
        str | None,
        typer.Option("--model", help="Model id (overrides STEMWISE_MODEL).", show_default=False),
    ] = None,
    temperature: Annotated[
        float | None,
        typer.Option(
            "--temperature",
            help="Sampling temperature (overrides STEMWISE_TEMPERATURE, default 0.2).",
            show_default=False,
        ),
    ] = None,
    reasoning_effort: Annotated[
        str | None,
        typer.Option(
            "--reasoning-effort",
            help=f"Reasoning effort: {', '.join(REASONING_EFFORTS)} "
            "(overrides STEMWISE_REASONING_EFFORT).",
            show_default=False,
        ),
    ] = None,
    max_iter: Annotated[
        int | None,
        typer.Option(
            "--max-iter",
            min=1,
            max=3,
            help="Maximum iterations of the loop (overrides options.max_iterations, default 3).",
            show_default=False,
        ),
    ] = None,
    export_stems: Annotated[
        bool,
        typer.Option("--export-stems", help="Also write the processed stems and reverb return."),
    ] = False,
) -> None:
    """Run the loop analysis → decision → render → evaluation (up to 3 iterations)."""
    if baseline_only and plan is not None:
        raise _fail("--baseline-only and --plan cannot be used together")
    if export_stems and baseline_only:
        raise _fail("--export-stems cannot be used with --baseline-only")
    decider_options = {
        "--decider": decider,
        "--model": model,
        "--temperature": temperature,
        "--reasoning-effort": reasoning_effort,
        "--max-iter": max_iter,
    }
    given = [name for name, value in decider_options.items() if value is not None]
    if given and (plan is not None or baseline_only):
        mode = "--plan" if plan is not None else "--baseline-only"
        raise _fail(f"{', '.join(given)} cannot be used with {mode} (no decision is made)")
    if plan is not None:
        _mix_with_plan(project, plan, export_stems)
        return
    if baseline_only:
        try:
            result = runs.baseline_run(project)
        except (StemwiseError, OSError) as exc:
            raise _fail(str(exc)) from exc
        typer.echo(f"run: {result.run_dir}")
        typer.echo(f"baseline: {result.baseline} ({_bus_summary(result.bus)})")
        return
    try:
        chosen = _make_decider(decider or DeciderName.llm, model, temperature, reasoning_effort)
        result = runs.mix_run(project, chosen, max_iterations=max_iter, export_stems=export_stems)
    except (StemwiseError, OSError) as exc:
        raise _fail(str(exc)) from exc
    _report_mix_run(result)


def _cost_summary(response_log: dict | None) -> str:
    if not response_log:
        return "no model call"
    total = response_log.get("total") or {}
    cost = total.get("cost_usd")
    cost_text = "cost not reported" if cost is None else f"cost {cost:.4f} USD"
    if cost is not None and not total.get("cost_complete", True):
        cost_text += " (partial)"
    return (
        f"{total.get('attempts', 0)} attempt(s), {total.get('prompt_tokens', 0)} prompt + "
        f"{total.get('completion_tokens', 0)} completion tokens, {cost_text}, "
        f"{total.get('duration_s', 0.0):.1f} s"
    )


def _distance(evaluation) -> str:
    d = evaluation.spectral_distance_db
    return "no target" if d is None else f"distance {d:.1f} dB"


def _constraints(evaluation) -> str:
    c = evaluation.constraints
    failed = [
        name
        for name, ok in (
            ("true peak", c.true_peak_ok),
            ("low correlation", c.low_corr_ok),
            ("DC offset", c.dc_ok),
        )
        if not ok
    ]
    return "constraints ok" if not failed else "violates: " + ", ".join(failed)


def _report_mix_run(result: runs.MixRun, *, show_decider: bool = True) -> None:
    typer.echo(f"run: {result.run_dir}")
    revision = result.revision
    if revision is not None:
        typer.echo(
            f"revision: from run {revision.source_run}, iter_{revision.source_iteration:02d} "
            f'(its best plan); request: "{revision.request}"'
        )
    typer.echo(
        f"analysis: {result.analysis_file} (~{estimated_tokens(result.analysis_text):.0f} tokens)"
    )
    baseline = result.baseline_evaluation
    bus = "copied" if result.baseline_bus is None else _bus_summary(result.baseline_bus)
    typer.echo(
        f"baseline: {result.baseline} ({bus}; {_distance(baseline)}, {_constraints(baseline)})"
    )
    for it in result.iterations:
        d = it.decision
        typer.echo(f"{it.dir.name}:")
        if show_decider:
            config = d.request_log.get("config") or {}
            who = d.request_log.get("decider", "?")
            if config.get("model"):
                who += f" ({config['model']})"
            typer.echo(f"  decider: {who}, {_cost_summary(d.response_log)}")
        else:
            pre_gains = ", ".join(f"{sid} {g:+.1f} dB" for sid, g in it.pre_gain_db.items())
            typer.echo(f"  pre-gain: {pre_gains}")
        typer.echo(f"  plan: {it.plan_file}")
        typer.echo(f"  summary: {d.plan.summary}")
        typer.echo(f"  mix: {it.mix_file} ({_bus_summary(it, 'before trim')})")
        ev = it.evaluation
        best = ", best so far" if ev.is_best and revision is None else ""
        typer.echo(f"  evaluation: {_distance(ev)}, {_constraints(ev)}{best}")
        if it.exported:
            typer.echo(f"  stems: {len(it.exported)} files in {it.exported[0].parent}")
    stop = result.best.stop_reason or "-"
    if revision is not None:
        if result.best.iteration is not None:
            typer.echo(f"revised plan: {result.best.plan} (no rollback: judge it by ear)")
    elif result.best.iteration is None:
        typer.echo(f"best: none within the constraints (stop: {stop})")
    else:
        typer.echo(f"best: {result.best.plan} (stop: {stop})")
    typer.echo(f"report: {result.report_file}")
    if result.failure is not None:
        typer.echo(
            f"warning: the decision of iteration {result.failed_iteration} failed; "
            "the iterations before it are kept",
            err=True,
        )
    if result.best.iteration is None and revision is not None:
        raise _fail(
            "the revised plan breaks a technical constraint (see best.json); its files are kept "
            f"for listening. To revise again from the starting plan: --run {revision.source_run}"
        )
    if result.best.iteration is None:
        raise _fail("no iteration satisfies the technical constraints (see best.json)")


def _bus_summary(bus: BusResult | MixResult | runs.IterationRun, before: str = "raw sum") -> str:
    trim = f"trim {bus.trim_gain_db:+.2f} dB" if bus.trim_gain_db else "no trim"
    peaks = f"true peak {bus.true_peak_db:.2f} dBTP, {before} {bus.true_peak_before_db:.2f} dBTP"
    return f"{peaks}, {trim}"


def _mix_with_plan(project: Path, plan: Path, export_stems: bool) -> None:
    try:
        result = runs.plan_run(project, plan, export_stems=export_stems)
    except (StemwiseError, OSError) as exc:
        raise _fail(str(exc)) from exc
    _report_mix_run(result, show_decider=False)


def _request(value: str) -> str:
    if not value.strip() or len(value.strip()) > MAX_REQUEST_CHARS:
        raise typer.BadParameter(f"the request must have 1 to {MAX_REQUEST_CHARS} characters")
    return value


@app.command()
def revise(
    project: ProjectArg,
    request: Annotated[
        str,
        typer.Argument(
            help="Revision request in natural language (top priority for the decider).",
            callback=_request,
            show_default=False,
        ),
    ],
    run_dir: Annotated[
        Path | None,
        typer.Option(
            "--run",
            help="Run to start from (default: the latest run of the project with evaluations).",
            show_default=False,
        ),
    ] = None,
    decider: Annotated[
        DeciderName | None,
        typer.Option(
            "--decider",
            help="Who decides the plan: the real model (llm, default) or the mock.",
            show_default=False,
        ),
    ] = None,
    model: Annotated[
        str | None,
        typer.Option("--model", help="Model id (overrides STEMWISE_MODEL).", show_default=False),
    ] = None,
    temperature: Annotated[
        float | None,
        typer.Option(
            "--temperature",
            help="Sampling temperature (overrides STEMWISE_TEMPERATURE, default 0.2).",
            show_default=False,
        ),
    ] = None,
    reasoning_effort: Annotated[
        str | None,
        typer.Option(
            "--reasoning-effort",
            help=f"Reasoning effort: {', '.join(REASONING_EFFORTS)} "
            "(overrides STEMWISE_REASONING_EFFORT).",
            show_default=False,
        ),
    ] = None,
    export_stems: Annotated[
        bool,
        typer.Option("--export-stems", help="Also write the processed stems and reverb return."),
    ] = False,
) -> None:
    """Run one iteration from the best plan of a run, driven by a request in words."""
    try:
        chosen = _make_decider(decider or DeciderName.llm, model, temperature, reasoning_effort)
        result = runs.revise_run(project, chosen, request, run=run_dir, export_stems=export_stems)
    except (StemwiseError, OSError) as exc:
        raise _fail(str(exc)) from exc
    _report_mix_run(result)


def _report_master_run(result: runs.MasterRun) -> None:
    r = result.report
    written = ", ".join(str(result.master_dir / name) for name in r.files)
    typer.echo(f"master: {written} ({r.sample_rate} Hz)")
    source = r.mix
    if source.run is None:
        typer.echo(f"mix: {result.mix_file} (file)")
    else:
        typer.echo(f"mix: run {source.run}, iter_{source.iteration:02d} ({result.mix_file})")
    ref = r.reference
    typer.echo(f"reference: {ref.id + ' ' if ref.id else ''}({result.reference_file})")
    typer.echo(
        f"loudness: master {_lufs(r.lufs)}, reference {_lufs(ref.lufs)}, mix {_lufs(source.lufs)}"
    )
    ceiling = "within" if r.true_peak_ok else "above"
    typer.echo(f"true peak: master {r.true_peak_db:+.1f} dBTP ({ceiling} -1.0 dBTP)")
    typer.echo(
        f"distance from the reference: mix {_db(source.spectral_distance_db)} -> "
        f"master {_db(r.spectral_distance_db)}"
    )
    typer.echo(f"report: {result.report_file}")


def _lufs(value: float | None) -> str:
    return "n/a" if value is None else f"{value:.1f} LUFS"


def _db(value: float | None) -> str:
    return "n/a" if value is None else f"{value:.1f} dB"


@app.command()
def master(
    project: ProjectArg,
    run_dir: Annotated[
        Path | None,
        typer.Option(
            "--run",
            help="Run whose best mix is mastered (default: the latest run with evaluations).",
            show_default=False,
        ),
    ] = None,
    mix: Annotated[
        Path | None,
        typer.Option(
            "--mix",
            help="Master this mix file instead (WAV or FLAC at the project rate; new run).",
            show_default=False,
        ),
    ] = None,
    reference: Annotated[
        str | None,
        typer.Option(
            "--reference",
            help="Reference: ref1, ref2, ref3 of project.yaml (default ref1) or a file path.",
            show_default=False,
        ),
    ] = None,
    pcm16: Annotated[
        bool,
        typer.Option("--pcm16", help="Also write a 16-bit copy (master_16.wav, no dither)."),
    ] = False,
) -> None:
    """Master the best mix against a reference track (MAS-1 → MAS-3)."""
    try:
        result = runs.master_run(project, run=run_dir, mix=mix, reference=reference, pcm16=pcm16)
    except (StemwiseError, OSError) as exc:
        raise _fail(str(exc)) from exc
    _report_master_run(result)


@app.command()
def report(
    project: ProjectArg,
    run_dir: Annotated[
        Path | None,
        typer.Option(
            "--run",
            help="Run folder (default: the latest run of the project with evaluations).",
            show_default=False,
        ),
    ] = None,
) -> None:
    """Regenerate report.md of the latest run (or of --run) from its files."""
    try:
        load_project(project)  # a clear error for a wrong project folder
        path = write_report(find_run(project, run_dir))
    except (StemwiseError, OSError) as exc:
        raise _fail(str(exc)) from exc
    typer.echo(f"report: {path}")


@app.command()
def ab(project: ProjectArg) -> None:
    """Blind listening files (REP-2): dropped from the PoC (D-079)."""
    raise _not_implemented("ab", "dropped from the PoC, D-079")


def run() -> None:
    """Console entry point: maps engine errors to exit code 1 without a traceback."""
    try:
        app()
    except StemwiseError as exc:  # pragma: no cover - commands already catch them
        typer.echo(f"error: {exc}", err=True)
        raise SystemExit(EXIT_ERROR) from exc
