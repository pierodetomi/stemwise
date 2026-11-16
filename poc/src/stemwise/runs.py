"""Run folders and runs: baseline (M0), analysis (M1), plan (M2), mix loop (M3, M4), revision (M5),
master (M6).

A run lives in `<project>/runs/<YYYY-MM-DDTHH-MM-SS>` (local time). Folders are created
exclusively: a run never overwrites a previous one, nor touches `stems/` and `references/`.
"""

from __future__ import annotations

import contextlib
import json
import logging
import shutil
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import numpy as np
from pydantic import ValidationError

from stemwise.analysis.analyze import analyze_loaded, analyze_loaded_details, project_target
from stemwise.analysis.model import Analysis, dumps_analysis
from stemwise.analysis.reference import Target
from stemwise.audio_io import (
    Stems,
    load_stems,
    probe,
    read_audio,
    resolve_sample_rate,
    write_wav16,
    write_wav24,
)
from stemwise.decision.base import Decider, Decision, DecisionInput, HistoryEntry, decision_input
from stemwise.decision.schema import MixPlan, check_plan_fits, dumps_plan, load_plan
from stemwise.errors import DecisionError
from stemwise.evaluate import Best, Evaluation, dumps_best, dumps_evaluation, evaluate_mix
from stemwise.loop import Iteration, revision_stop, run_loop, stop_reason
from stemwise.master import (
    MasterError,
    MasterReport,
    MixSource,
    build_report,
    dumps_master_report,
    load_master_reference,
    load_mix,
    master_audio,
    select_mix,
    select_reference,
)
from stemwise.project import Project, load_project
from stemwise.render.mixbus import BusResult, render_baseline
from stemwise.render.render import MixResult, render_mix
from stemwise.report import find_run, write_report
from stemwise.revise import (
    MAX_REQUEST_CHARS,
    Revision,
    RevisionError,
    check_consistency,
    dumps_revision,
    load_source,
)

log = logging.getLogger(__name__)

RUNS_DIR = "runs"
RUN_NAME_FORMAT = "%Y-%m-%dT%H-%M-%S"
BASELINE_FILE = "baseline.wav"
ANALYSIS_FILE = "analysis.json"
PLAN_FILE = "plan.json"
MIX_FILE = "mix.wav"
STEMS_DIR = "stems"
REVERB_RETURN_FILE = "reverb_return.wav"
LLM_REQUEST_FILE = "llm_request.json"
LLM_RESPONSE_FILE = "llm_response.json"
EVALUATION_FILE = "evaluation.json"
BASELINE_EVALUATION_FILE = "baseline_evaluation.json"
BEST_FILE = "best.json"
REVISION_FILE = "revision.json"
SOURCE_PLAN_FILE = "source_plan.json"
SOURCE_EVALUATION_FILE = "source_evaluation.json"
MASTER_DIR = "master"
MASTER_FILE = "master.wav"
MASTER_16_FILE = "master_16.wav"
MASTER_REPORT_FILE = "master_report.json"
_MAX_SUFFIX = 1000


def now() -> datetime:
    """Current local time (a function, so tests can replace it)."""
    return datetime.now()


def create_run_dir(project_root: Path, when: datetime) -> Path:
    """Create a new, empty run folder; on a name clash append `-2`, `-3`, ..."""
    runs = Path(project_root) / RUNS_DIR
    runs.mkdir(exist_ok=True)
    base = when.strftime(RUN_NAME_FORMAT)
    for i in range(1, _MAX_SUFFIX + 1):
        candidate = runs / (base if i == 1 else f"{base}-{i}")
        try:
            candidate.mkdir()
        except FileExistsError:
            continue
        return candidate
    raise FileExistsError(f"too many runs named {base} in {runs}")


@dataclass(frozen=True)
class BaselineRun:
    run_dir: Path
    baseline: Path
    sample_rate: int
    bus: BusResult


def baseline_run(project_dir: Path, when: datetime | None = None) -> BaselineRun:
    """`mix --baseline-only`: validate the project, sum the stems, write `baseline.wav` (REN-7).

    The run folder is created only after the project and the stems have been validated, and is
    removed again if writing fails, so a failing command leaves no trace in `runs/`.
    """
    project = load_project(project_dir)
    stems = load_stems(project)
    bus = render_baseline(stems, project.spec.options.mix_true_peak_db)
    run_dir = create_run_dir(project.root, when or now())
    try:
        baseline = write_wav24(run_dir / BASELINE_FILE, bus.audio, stems.sample_rate)
    except BaseException:
        shutil.rmtree(run_dir, ignore_errors=True)  # the folder is new: nothing else lives there
        raise
    return BaselineRun(run_dir, baseline, stems.sample_rate, bus)


@dataclass(frozen=True)
class AnalysisRun:
    run_dir: Path
    analysis_file: Path
    analysis: Analysis
    text: str


def analysis_run(project_dir: Path, when: datetime | None = None) -> AnalysisRun:
    """`analyze`: validate and analyse the project, write `analysis.json` in a new run (D-019).

    As for the baseline run, the folder is created only once the analysis has succeeded and is
    removed again if writing fails.
    """
    project = load_project(project_dir)
    analysis = analyze_loaded(project)
    text = dumps_analysis(analysis)
    run_dir = create_run_dir(project.root, when or now())
    path = run_dir / ANALYSIS_FILE
    try:
        with path.open("x", encoding="utf-8") as fh:
            fh.write(text)
    except BaseException:
        shutil.rmtree(run_dir, ignore_errors=True)  # the folder is new: nothing else lives there
        raise
    return AnalysisRun(run_dir, path, analysis, text)


def _warn_clipped_tracks(tracks: dict[str, np.ndarray | None]) -> None:
    """Exported tracks above full scale are clipped by the 24-bit file: then they no longer sum
    to the mix (D-032). The mix itself is always within the ceiling (REN-5)."""
    for name, audio in tracks.items():
        if audio is not None and audio.size and float(np.max(np.abs(audio))) > 1.0:
            log.warning(
                "exported track '%s' peaks above 0 dBFS (%.2f dBFS): its file is clipped, so the "
                "exported tracks no longer sum exactly to mix.wav",
                name,
                20.0 * np.log10(float(np.max(np.abs(audio)))),
            )


def iteration_dir_name(iteration: int) -> str:
    return f"iter_{iteration:02d}"


def _write_json(path: Path, doc: object) -> Path:
    with path.open("x", encoding="utf-8") as fh:
        fh.write(json.dumps(doc, indent=2, ensure_ascii=False) + "\n")
    return path


def _write_json_text(path: Path, text: str) -> Path:
    with path.open("x", encoding="utf-8") as fh:
        fh.write(text)
    return path


def _replace_text(path: Path, text: str) -> Path:
    """Write `path` through a temporary file, so a reader never sees half a file (best.json)."""
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(text, encoding="utf-8")
    tmp.replace(path)
    return path


def _write_decision_logs(
    iter_dir: Path, request_log: dict | None, response_log: dict | None
) -> list[Path]:
    written = []
    if request_log:
        written.append(_write_json(iter_dir / LLM_REQUEST_FILE, request_log))
    if response_log:
        written.append(_write_json(iter_dir / LLM_RESPONSE_FILE, response_log))
    return written


@dataclass(frozen=True)
class IterationRun:
    """The files and the numbers of one iteration of the loop (PRD §8)."""

    iteration: int
    dir: Path
    plan_file: Path
    mix_file: Path
    evaluation_file: Path
    logs: list[Path]
    exported: list[Path]
    decision: Decision
    evaluation: Evaluation
    pre_gain_db: dict[str, float]
    true_peak_before_db: float
    trim_gain_db: float
    true_peak_db: float


@dataclass(frozen=True)
class MixRun:
    run_dir: Path
    analysis_file: Path
    analysis_text: str
    baseline: Path
    baseline_bus: BusResult | None  # None for a revision: its baseline.wav is a copy (D-061)
    baseline_evaluation: Evaluation
    iterations: list[IterationRun]
    best: Best
    best_file: Path
    report_file: Path
    failure: DecisionError | None = None  # decision that stopped the loop after iteration 0
    failed_iteration: int | None = None
    revision: Revision | None = None  # set for a revision run (D-059)

    @property
    def best_iteration(self) -> IterationRun | None:
        if self.best.iteration is None:
            return None
        return self.iterations[self.best.iteration]


class _FixedPlan:
    """`mix --plan`: a "decider" that returns the author's plan, once (D-026)."""

    name = "plan"

    def __init__(self, plan: MixPlan) -> None:
        self.plan = plan

    def decide(self, decision_input: DecisionInput) -> Decision:
        return Decision(plan=self.plan)


class _Rendered:
    """A rendered mix held only until its files are written, so the loop keeps no audio."""

    def __init__(self, result: MixResult) -> None:
        self.result: MixResult | None = result


@dataclass(frozen=True)
class _Start:
    """What the loop of a run (`mix` or `revise`) starts from, known before its folder exists."""

    stems: Stems
    analysis_text: str
    target: Target | None
    baseline_evaluation: Evaluation
    baseline_bus: BusResult | None
    first_input: DecisionInput
    write: Callable[[Path], tuple[Path, Path]]  # first files of the run: (analysis, baseline)


def mix_run(
    project_dir: Path,
    decider: Decider,
    *,
    max_iterations: int | None = None,
    export_stems: bool = False,
    when: datetime | None = None,
    plan: MixPlan | None = None,
) -> MixRun:
    """`mix`: analysis, raw sum, then the loop decision → render → evaluation (PRD §4, M4).

    Writes `analysis.json`, `baseline.wav`, `baseline_evaluation.json` and, for every iteration,
    `iter_NN/` (`plan.json`, `llm_request.json`, `llm_response.json` when a model was called,
    `mix.wav`, `evaluation.json`, exported stems on request); `best.json` is rewritten after
    every iteration (D-049); `report.md` at the end (REP-1). `max_iterations` overrides
    `options.max_iterations` (D-051).

    A failed decision on iteration 0 behaves as in M3: the folder is kept with its logs when the
    model was called (D-040), otherwise removed. A failure on a later iteration stops the loop
    and keeps everything (D-055). `plan` is used by `plan_run` and must already fit the project.
    """
    project = load_project(project_dir)
    preflight = getattr(decider, "preflight", None)
    if preflight is not None:
        preflight()  # e.g. the model exists: checked before the analysis (D-044)
    stems = load_stems(project)
    if plan is not None:
        check_plan_fits(plan, list(stems.audio), stems.sample_rate)
    details = analyze_loaded_details(project, stems)
    text = dumps_analysis(details.analysis)
    options = project.spec.options
    ceiling = options.mix_true_peak_db
    max_iter = options.max_iterations if max_iterations is None else max_iterations
    fs = stems.sample_rate
    baseline_bus = render_baseline(stems, ceiling)
    baseline_evaluation = evaluate_mix(
        baseline_bus.audio,
        fs,
        details.stem_spectra,
        details.target,
        ceiling_db=ceiling,
        measures=details.mix,
    )

    def write(run_dir: Path) -> tuple[Path, Path]:
        analysis_file = _write_json_text(run_dir / ANALYSIS_FILE, text)
        baseline = write_wav24(run_dir / BASELINE_FILE, baseline_bus.audio, fs)
        _write_json_text(run_dir / BASELINE_EVALUATION_FILE, dumps_evaluation(baseline_evaluation))
        return analysis_file, baseline

    start = _Start(
        stems=stems,
        analysis_text=text,
        target=details.target,
        baseline_evaluation=baseline_evaluation,
        baseline_bus=baseline_bus,
        first_input=decision_input(project, details.analysis, text, iteration=0),
        write=write,
    )
    return _loop_run(
        project,
        _FixedPlan(plan) if plan is not None else decider,
        start,
        max_iterations=1 if plan is not None else max_iter,
        export_stems=export_stems,
        when=when,
        interrupted_decider=decider,
    )


def revise_run(
    project_dir: Path,
    decider: Decider,
    request: str,
    *,
    run: Path | None = None,
    export_stems: bool = False,
    when: datetime | None = None,
) -> MixRun:
    """`revise` (REV-1 → REV-3, D-059 → D-063): one iteration from the best plan of a run.

    The source is the latest run with `best.json`, or `run` (a name in `runs/`, then a path).
    Its analysis, raw sum and the evaluation of the starting plan are copied into a new run;
    only the references are measured again, for the target, after checking that stems and
    references are still those of the source (D-061). The decider sees the starting plan with
    its evaluation and the request (D-062); DEC-5 limits the change from the starting plan
    (D-063). `best.json` names `iter_00` whatever its distance, if it is within the technical
    constraints (no rollback, REV-2, D-060). Errors behave as for `mix`.
    """
    project = load_project(project_dir)
    source = load_source(project.root, find_run(project.root, run))
    try:
        revision = Revision(
            source_run=source.label,
            source_iteration=source.iteration,
            source_plan=source.plan_path,
            request=request,
        )
    except ValidationError as exc:
        raise RevisionError(
            f"the request must contain from 1 to {MAX_REQUEST_CHARS} characters"
        ) from exc
    preflight = getattr(decider, "preflight", None)
    if preflight is not None:
        preflight()  # D-044: before loading the audio
    stems = load_stems(project)
    target = project_target(project, stems.sample_rate)
    check_consistency(source, stems, {s.id: s.role for s in project.spec.stems}, target)
    history = (HistoryEntry(source.plan, source.evaluation.model_dump(mode="json")),)
    first_input = decision_input(
        project,
        source.analysis,
        source.analysis_text,
        iteration=0,
        history=history,
        request=revision.request,
        previous_plan=source.plan,
    )

    def write(run_dir: Path) -> tuple[Path, Path]:
        _write_json_text(run_dir / REVISION_FILE, dumps_revision(revision))
        _write_json_text(
            run_dir / SOURCE_PLAN_FILE,
            (source.run_dir / source.plan_path).read_text(encoding="utf-8"),
        )
        _write_json_text(run_dir / SOURCE_EVALUATION_FILE, source.evaluation_text)
        analysis_file = _write_json_text(run_dir / ANALYSIS_FILE, source.analysis_text)
        baseline = run_dir / BASELINE_FILE
        shutil.copyfile(source.baseline_file, baseline)
        _write_json_text(run_dir / BASELINE_EVALUATION_FILE, source.baseline_evaluation_text)
        return analysis_file, baseline

    start = _Start(
        stems=stems,
        analysis_text=source.analysis_text,
        target=target,
        baseline_evaluation=source.baseline_evaluation,
        baseline_bus=None,
        first_input=first_input,
        write=write,
    )
    return _loop_run(
        project,
        decider,
        start,
        max_iterations=1,
        export_stems=export_stems,
        when=when,
        stop_rule=revision_stop,
        revision=revision,
    )


def _loop_run(
    project: Project,
    decider: Decider,
    start: _Start,
    *,
    max_iterations: int,
    export_stems: bool,
    when: datetime | None,
    stop_rule=stop_reason,
    revision: Revision | None = None,
    interrupted_decider: Decider | None = None,
) -> MixRun:
    """Create the run folder, write the first files, run the loop and write the results."""
    stems = start.stems
    options = project.spec.options
    ceiling = options.mix_true_peak_db
    fs = stems.sample_rate
    run_dir = create_run_dir(project.root, when or now())
    iterations: list[IterationRun] = []
    decided: dict[int, tuple[Decision, list[Path], Path]] = {}
    called = False

    def on_decision(k: int, decision: Decision) -> None:
        nonlocal called
        iter_dir = run_dir / iteration_dir_name(k)
        iter_dir.mkdir()
        request_log = (
            {**decision.request_log, "warnings": list(decision.warnings)}
            if decision.request_log
            else None
        )
        logs = _write_decision_logs(iter_dir, request_log, decision.response_log)
        called = called or decision.response_log is not None  # a model was called: keep it
        plan_file = _write_json_text(iter_dir / PLAN_FILE, dumps_plan(decision.plan))
        decided[k] = (decision, logs, plan_file)

    def render(plan_: MixPlan) -> _Rendered:
        return _Rendered(
            render_mix(
                stems,
                plan_,
                pre_normalize=options.pre_normalize,
                ceiling_db=ceiling,
                keep_tracks=export_stems,
                track_spectra=True,
            )
        )

    def evaluate(rendered: _Rendered, k: int) -> Evaluation:
        mix = rendered.result
        assert mix is not None
        return evaluate_mix(
            mix.audio, fs, mix.track_spectra, start.target, ceiling_db=ceiling, iteration=k
        )

    def on_iteration(it: Iteration[_Rendered], best: Best) -> None:
        mix = it.mix.result
        assert mix is not None
        decision, logs, plan_file = decided[it.iteration]
        iter_dir = plan_file.parent
        tracks = mix.exported_tracks()
        reverb_return = mix.exported_reverb_return()
        _warn_clipped_tracks({**tracks, "reverb return": reverb_return})
        mix_file = write_wav24(iter_dir / MIX_FILE, mix.audio, fs)
        exported = [
            write_wav24(iter_dir / STEMS_DIR / f"{name}.wav", audio, fs)
            for name, audio in tracks.items()
        ]
        if reverb_return is not None:
            exported.append(write_wav24(iter_dir / REVERB_RETURN_FILE, reverb_return, fs))
        evaluation_file = _write_json_text(
            iter_dir / EVALUATION_FILE, dumps_evaluation(it.evaluation)
        )
        _replace_text(run_dir / BEST_FILE, dumps_best(best))
        iterations.append(
            IterationRun(
                iteration=it.iteration,
                dir=iter_dir,
                plan_file=plan_file,
                mix_file=mix_file,
                evaluation_file=evaluation_file,
                logs=logs,
                exported=exported,
                decision=decision,
                evaluation=it.evaluation,
                pre_gain_db=mix.pre_gain_db,
                true_peak_before_db=mix.true_peak_before_db,
                trim_gain_db=mix.trim_gain_db,
                true_peak_db=mix.true_peak_db,
            )
        )
        it.mix.result = None  # the files are written: drop the audio

    keep = False
    try:
        analysis_file, baseline = start.write(run_dir)
        try:
            result = run_loop(
                decider,
                start.first_input,
                render=render,
                evaluate=evaluate,
                max_iterations=max_iterations,
                on_decision=on_decision,
                on_iteration=on_iteration,
                stop_rule=stop_rule,
            )
        except DecisionError as exc:  # iteration 0 (D-040)
            iter_dir = run_dir / iteration_dir_name(0)
            iter_dir.mkdir(exist_ok=True)
            _write_decision_logs(iter_dir, exc.request_log, exc.response_log)
            keep = bool((exc.request_log or {}).get("attempts"))
            if not keep:
                raise
            raise DecisionError(
                f"{exc}\nrun folder kept with the logs of the calls: {run_dir}",
                request_log=exc.request_log,
                response_log=exc.response_log,
            ) from exc
        keep = True
        if result.failure is not None:  # D-055: a later decision failed, the result stays
            k = result.failed_iteration
            iter_dir = run_dir / iteration_dir_name(k)
            iter_dir.mkdir(exist_ok=True)
            _write_decision_logs(iter_dir, result.failure.request_log, result.failure.response_log)
            log.warning(
                "the decision of iteration %d failed, the loop stopped: %s", k, result.failure
            )
        best_file = _replace_text(run_dir / BEST_FILE, dumps_best(result.best))
        report_file = write_report(run_dir)  # REP-1, from the files just written
    except DecisionError:
        if not keep:
            shutil.rmtree(run_dir, ignore_errors=True)  # new folder: nothing else lives there
        raise
    except BaseException:
        current = run_dir / iteration_dir_name(len(iterations))
        saved = _save_interrupted_logs(
            interrupted_decider or decider, current
        )  # always: earlier calls do not cover it
        keep = keep or called or saved
        if keep:
            log.error("run folder kept with the logs of the calls: %s", run_dir)
        else:
            shutil.rmtree(run_dir, ignore_errors=True)  # new folder: nothing else lives there
        raise
    return MixRun(
        run_dir=run_dir,
        analysis_file=analysis_file,
        analysis_text=start.analysis_text,
        baseline=baseline,
        baseline_bus=start.baseline_bus,
        baseline_evaluation=start.baseline_evaluation,
        iterations=iterations,
        best=result.best,
        best_file=best_file,
        report_file=report_file,
        failure=result.failure,
        failed_iteration=result.failed_iteration,
        revision=revision,
    )


def plan_run(
    project_dir: Path, plan_path: Path, *, export_stems: bool = False, when: datetime | None = None
) -> MixRun:
    """`mix --plan`: render and evaluate a hand-written plan (D-026) as a one-iteration run.

    The plan is read and checked against the project before any analysis; the written plan has
    `iteration` 0, the folder it lives in. Writes the same files as a decided run (M4, D-056).
    """
    plan = load_plan(plan_path).model_copy(update={"iteration": 0})
    return mix_run(project_dir, _FixedPlan(plan), export_stems=export_stems, when=when, plan=plan)


def _save_interrupted_logs(decider: Decider, iter_dir: Path) -> bool:
    """D-040: a decision interrupted after paid calls (Ctrl-C, ...) still leaves its logs."""
    request_log = getattr(decider, "last_request_log", None)
    if not request_log or not request_log.get("attempts"):
        return False
    if not (iter_dir / LLM_REQUEST_FILE).exists():
        with contextlib.suppress(OSError):
            iter_dir.mkdir(exist_ok=True)
            _write_decision_logs(iter_dir, request_log, getattr(decider, "last_response_log", None))
    return True


# --- master (M6, MAS-1 → MAS-3, D-072 → D-076) --------------------------------------------------


@dataclass(frozen=True)
class MasterRun:
    run_dir: Path
    master_dir: Path
    mix_file: Path
    reference_file: Path
    files: list[Path]
    report_file: Path
    report: MasterReport


def _project_sample_rate(project: Project) -> int:
    """The project rate from the stem headers, without decoding the audio (ING-3, D-010)."""
    rates = {s.id: probe(project.stem_path(s)).sample_rate for s in project.spec.stems}
    return resolve_sample_rate(rates, project.spec.sample_rate)


def create_master_dir(run_dir: Path) -> Path:
    """D-072: `master/`, or `master_02/`, `master_03/`... if taken; created exclusively."""
    for i in range(1, _MAX_SUFFIX + 1):
        candidate = Path(run_dir) / (MASTER_DIR if i == 1 else f"{MASTER_DIR}_{i:02d}")
        try:
            candidate.mkdir()
        except FileExistsError:
            continue
        return candidate
    raise FileExistsError(f"too many masters in {run_dir}")


def master_run(
    project_dir: Path,
    *,
    run: Path | None = None,
    mix: Path | None = None,
    reference: str | None = None,
    pcm16: bool = False,
    when: datetime | None = None,
) -> MasterRun:
    """`master` (MAS-1 → MAS-3): master a mix against a reference and measure the result.

    The mix is the best one of the latest run with `best.json`, or of `run`, and the master goes
    in a new `master/` (or `master_NN/`) folder of that run; a `mix` file instead gets a new run
    with only `master/` (D-072, D-073). The reference is `ref1` or `reference` (an id or a path).
    Folders are created only once the master is computed, and removed if writing fails.
    """
    if run is not None and mix is not None:
        raise MasterError("--run and --mix cannot be used together: --mix masters a file")
    project = load_project(project_dir)
    sample_rate = _project_sample_rate(project)
    if mix is None:
        chosen = select_mix(project.root, run)
        source_run: Path | None = chosen.run_dir
        mix_path, mix_source = chosen.path, chosen.source
    else:
        source_run = None
        mix_path, mix_source = Path(mix), MixSource(run=None, iteration=None, file=str(mix))
    ref = select_reference(project, reference)
    mix_audio = load_mix(mix_path, sample_rate)
    ref_audio = load_master_reference(ref.path, sample_rate)
    mastered = master_audio(mix_audio, ref_audio, sample_rate)

    if source_run is None:
        run_dir = create_run_dir(project.root, when or now())
        created = run_dir
        master_dir = run_dir / MASTER_DIR
        master_dir.mkdir()
    else:
        run_dir = source_run
        master_dir = created = create_master_dir(run_dir)
    try:
        files = [write_wav24(master_dir / MASTER_FILE, mastered, sample_rate)]
        if pcm16:
            files.append(write_wav16(master_dir / MASTER_16_FILE, mastered, sample_rate))
        written, _ = read_audio(files[0])  # D-076: measure the file as delivered
        report = build_report(
            sample_rate=sample_rate,
            master=written,
            mix=mix_audio,
            reference=ref_audio,
            mix_source=mix_source,
            reference_source=ref.source,
            files=[p.name for p in files],
        )
        report_file = _write_json_text(master_dir / MASTER_REPORT_FILE, dumps_master_report(report))
    except BaseException:
        shutil.rmtree(created, ignore_errors=True)  # new folder: nothing else lives there
        raise
    for warning in report.warnings:
        log.warning(warning)
    return MasterRun(
        run_dir=run_dir,
        master_dir=master_dir,
        mix_file=mix_path,
        reference_file=ref.path,
        files=files,
        report_file=report_file,
        report=report,
    )
