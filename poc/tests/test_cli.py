"""CLI wrapper (PRD §8): command set, exit codes, error reporting."""

from __future__ import annotations

import hashlib
import json
import math
from datetime import datetime
from pathlib import Path

import numpy as np
import pytest
import soundfile as sf
from typer.testing import CliRunner

from stemwise import runs
from stemwise.analysis.spectral import BANDS_HZ
from stemwise.analysis.stem import true_peak_db
from stemwise.audio_io import read_audio
from stemwise.cli import EXIT_ERROR, EXIT_NOT_IMPLEMENTED, app
from stemwise.decision.schema import load_plan
from stemwise.render.mixbus import sum_bus
from tests.helpers.plan import example_plan, neutral_plan, write_plan
from tests.helpers.project import StemFixture, make_project, write_audio
from tests.helpers.signals import SR, fade, noise, pink_noise, sine

runner = CliRunner()

COMMANDS = ["analyze", "mix", "revise", "master", "report", "ab"]


def test_help_lists_all_commands() -> None:
    result = runner.invoke(app, ["--help"])
    assert result.exit_code == 0
    for command in COMMANDS:
        assert command in result.stdout


def test_version() -> None:
    result = runner.invoke(app, ["--version"])
    assert result.exit_code == 0
    assert result.stdout.startswith("stemwise ")


@pytest.mark.parametrize(
    "args",
    [
        ["ab", "p"],
    ],
)
def test_later_milestone_commands_exit_non_zero(args: list[str]) -> None:
    result = runner.invoke(app, args)
    assert result.exit_code == EXIT_NOT_IMPLEMENTED
    assert "not implemented yet" in result.stderr


# --- mix --baseline-only (M0 acceptance) ---------------------------------------------------------


def _loud_stems() -> list[StemFixture]:
    """In-phase float stems whose raw sum peaks well above 0 dBTP, plus a mono stem."""
    return [
        StemFixture("kick", "kick", fade(sine(60, amplitude=0.6)), subtype="FLOAT"),
        StemFixture("bass", "bass", fade(sine(60, amplitude=0.6, channels=2)), subtype="FLOAT"),
        StemFixture("keys", "keys", fade(sine(60, amplitude=0.6, channels=2)), subtype="FLOAT"),
        StemFixture("vox", "lead_vocal", fade(noise(amplitude=0.05, seed=9)), ext="flac"),
    ]


def _paths(tmp_path: Path, stems: list[StemFixture]) -> list[tuple[StemFixture, Path]]:
    """Encode the non-float stems once (as the project will), to measure what the CLI reads."""
    out = []
    for s in stems:
        path = tmp_path / "encoded" / f"{s.id}.{s.ext}"
        write_audio(path, s.data, s.sr, s.subtype)
        out.append((s, path))
    return out


def _digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _tree_digests(root: Path) -> dict[str, str]:
    return {str(p.relative_to(root)): _digest(p) for p in sorted(root.rglob("*")) if p.is_file()}


def _baselines(root: Path) -> list[Path]:
    return sorted((root / "runs").glob("*/baseline.wav"))


@pytest.mark.acceptance("M0")
@pytest.mark.req("REN-5", "REN-7", "ING-6")
def test_baseline_only_within_true_peak_limit(tmp_path: Path) -> None:
    stems = _loud_stems()
    # Precondition: the raw sum of the stems is above the ceiling, so the trim must act.
    raw_sum = sum_bus(
        [s.data if s.subtype == "FLOAT" else read_audio(p)[0] for s, p in _paths(tmp_path, stems)]
    )
    assert true_peak_db(raw_sum) > 0.0
    root = make_project(tmp_path / "song", stems)
    result = runner.invoke(app, ["mix", str(root), "--baseline-only"])
    assert result.exit_code == 0, result.stderr
    assert "trim -" in result.stdout
    [baseline] = _baselines(root)
    info = sf.info(baseline)
    assert (info.format, info.subtype, info.channels, info.samplerate) == ("WAV", "PCM_24", 2, SR)
    assert info.frames == SR  # same duration as the stems (1 s)
    audio, _ = read_audio(baseline)
    assert true_peak_db(audio) <= -3.0


@pytest.mark.acceptance("M0")
@pytest.mark.req("ING-4")
def test_baseline_only_pads_with_warning(tmp_path: Path) -> None:
    stems = [
        StemFixture("kick", "kick", sine(60, duration_s=1.0, amplitude=0.2)),
        StemFixture("bass", "bass", sine(110, duration_s=0.6, amplitude=0.2, channels=2)),
    ]
    root = make_project(tmp_path / "song", stems)
    result = runner.invoke(app, ["mix", str(root), "--baseline-only"])
    assert result.exit_code == 0, result.stderr
    assert "warning:" in result.stderr
    assert "bass" in result.stderr
    [baseline] = _baselines(root)
    assert sf.info(baseline).frames == SR  # padded to the longest stem


@pytest.mark.acceptance("M0")
@pytest.mark.req("ING-3")
def test_baseline_only_sample_rate_mismatch_fails(tmp_path: Path) -> None:
    stems = [
        StemFixture("kick", "kick", sine(60, amplitude=0.2)),
        StemFixture("bass", "bass", sine(110, sr=44_100, amplitude=0.2), sr=44_100),
    ]
    root = make_project(tmp_path / "song", stems)
    result = runner.invoke(app, ["mix", str(root), "--baseline-only"])
    assert result.exit_code == EXIT_ERROR
    assert "sample rate" in result.stderr
    assert "Traceback" not in result.stderr
    assert not (root / "runs").exists()  # nothing written on failure


@pytest.mark.acceptance("M0")
@pytest.mark.req("REN-7")
def test_runs_never_overwrite(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    root = make_project(
        tmp_path / "song", _loud_stems(), references=[("ref.wav", sine(440, amplitude=0.3), SR)]
    )
    inputs_before = {d: _tree_digests(root / d) for d in ("stems", "references")}
    monkeypatch.setattr(runs, "now", lambda: datetime(2026, 11, 9, 21, 30, 0))

    assert runner.invoke(app, ["mix", str(root), "--baseline-only"]).exit_code == 0
    [first] = _baselines(root)
    first_digest = _digest(first)
    assert runner.invoke(app, ["mix", str(root), "--baseline-only"]).exit_code == 0

    assert [p.parent.name for p in _baselines(root)] == [
        "2026-11-09T21-30-00",
        "2026-11-09T21-30-00-2",
    ]
    assert _digest(first) == first_digest
    # Same stems -> same file (deterministic raw sum, no time in the audio content).
    assert _digest(_baselines(root)[1]) == first_digest
    assert {d: _tree_digests(root / d) for d in ("stems", "references")} == inputs_before


def test_mix_reports_project_errors(tmp_path: Path) -> None:
    result = runner.invoke(app, ["mix", str(tmp_path), "--baseline-only"])
    assert result.exit_code == EXIT_ERROR
    assert "project.yaml not found" in result.stderr


def test_not_implemented_differs_from_usage_error(tmp_path: Path) -> None:
    usage = runner.invoke(app, ["mix", str(tmp_path), "--no-such-option"])
    assert usage.exit_code != EXIT_NOT_IMPLEMENTED


def test_write_failure_leaves_no_run_dir(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    root = make_project(tmp_path / "song")

    def failing_write(*args, **kwargs):
        raise PermissionError("read-only disk")

    monkeypatch.setattr(runs, "write_wav24", failing_write)
    result = runner.invoke(app, ["mix", str(root), "--baseline-only"])
    assert result.exit_code == EXIT_ERROR
    assert "read-only disk" in result.stderr
    assert "Traceback" not in result.stderr
    assert list((root / "runs").iterdir()) == []


# --- analyze (M1) --------------------------------------------------------------------------------


def _analyses(root: Path) -> list[Path]:
    return sorted((root / "runs").glob("*/analysis.json"))


def _analyze(root: Path) -> dict:
    result = runner.invoke(app, ["analyze", str(root)])
    assert result.exit_code == 0, result.stderr
    assert "analysis:" in result.stdout
    return json.loads(_analyses(root)[-1].read_text(encoding="utf-8"))


@pytest.mark.req("ANA-13")
def test_analyze_writes_run(tmp_path: Path) -> None:
    root = make_project(tmp_path / "song", references=[("ref.wav", sine(440, amplitude=0.3), SR)])
    inputs_before = {d: _tree_digests(root / d) for d in ("stems", "references")}
    doc = _analyze(root)
    assert list(doc["stems"]) == ["kick", "bass", "gtr", "lead_vox"]
    assert doc["project"] == {"sample_rate": SR, "duration_s": 1.0}
    assert len(_analyses(root)) == 1
    assert {d: _tree_digests(root / d) for d in ("stems", "references")} == inputs_before


def test_analyze_invalid_project_fails(tmp_path: Path) -> None:
    stems = [
        StemFixture("kick", "kick", sine(60, amplitude=0.2)),
        StemFixture("bass", "bass", sine(110, sr=44_100, amplitude=0.2), sr=44_100),
    ]
    root = make_project(tmp_path / "song", stems)
    result = runner.invoke(app, ["analyze", str(root)])
    assert result.exit_code == EXIT_ERROR
    assert "sample rate" in result.stderr
    assert "Traceback" not in result.stderr
    assert not (root / "runs").exists()


_ACCEPTANCE_S = 30.0  # long enough for a stable pink-noise band estimate


def _acceptance_project(root: Path) -> Path:
    twin = pink_noise(duration_s=_ACCEPTANCE_S, channels=2, rms=0.05, seed=5)
    stems = [
        StemFixture("tone", "keys", sine(1000, amplitude=0.5, duration_s=_ACCEPTANCE_S)),
        StemFixture("pink", "pad", pink_noise(duration_s=_ACCEPTANCE_S, rms=0.1)),
        StemFixture("twin_a", "backing_vocal", twin),
        StemFixture("twin_b", "backing_vocal", twin.copy()),
    ]
    return make_project(root, stems)


@pytest.fixture(scope="module")
def acceptance_doc(tmp_path_factory: pytest.TempPathFactory) -> dict:
    return _analyze(_acceptance_project(tmp_path_factory.mktemp("m1") / "song"))


@pytest.mark.acceptance("M1")
@pytest.mark.req("ANA-3")
def test_analyze_sine_1k_in_band_500_2k(acceptance_doc: dict) -> None:
    bands = acceptance_doc["stems"]["tone"]["bands_db"]
    assert bands[4] == 0.0  # 500-2k holds all the energy (rounded to 0.1 dB)
    assert all(b <= -60.0 for i, b in enumerate(bands) if i != 4)


@pytest.mark.acceptance("M1")
@pytest.mark.req("ANA-3")
def test_analyze_pink_noise_octave_bands(acceptance_doc: dict) -> None:
    bands = acceptance_doc["stems"]["pink"]["bands_db"]
    one_octave = [bands[i] for i in (1, 3, 5, 6)]  # 60-120, 250-500, 2k-4k, 4k-8k
    # Measured spread 0.19 dB before rounding; ±0.05 per value from rounding to 0.1 dB.
    assert max(one_octave) - min(one_octave) <= 0.3
    for value, (lo, hi) in zip(bands, BANDS_HZ, strict=True):
        assert value == pytest.approx(
            10 * math.log10(math.log2(hi / lo) / math.log2(1000)), abs=0.5
        )


@pytest.mark.acceptance("M1")
@pytest.mark.req("ANA-9")
def test_analyze_identical_stems_max_masking(acceptance_doc: dict) -> None:
    top = acceptance_doc["masking_top"]
    twins = [c for c in top if {c["a"], c["b"]} == {"twin_a", "twin_b"}]
    assert [c["band"] for c in twins] == list(
        ("20-60", "60-120", "120-250", "250-500", "500-2k", "2k-4k", "4k-8k", "8k-20k")
    )
    assert all(c["score"] == 1.0 for c in twins)
    assert top[: len(twins)] == twins  # at the head of the list
    others = [c for c in top if c not in twins]
    assert others
    assert all(c["score"] < 1.0 for c in others)


@pytest.mark.acceptance("M1")
@pytest.mark.req("ANA-13")
def test_analyze_12_stems_token_budget(tmp_path: Path) -> None:
    roles = ["kick", "snare", "toms", "overheads", "bass", "electric_guitar"] * 2
    stems = [
        StemFixture(
            f"stem_{i:02d}_{role}", role, pink_noise(duration_s=3.0, channels=2, seed=i, rms=0.05)
        )
        for i, role in enumerate(roles)
    ]
    refs = [
        (f"reference_{i}.wav", pink_noise(duration_s=3.0, channels=2, seed=50 + i), SR)
        for i in range(3)
    ]
    root = make_project(tmp_path / "song", stems, references=refs)
    _analyze(root)
    text = _analyses(root)[-1].read_text(encoding="utf-8")
    doc = json.loads(text)
    assert len(doc["stems"]) == 12
    assert len(doc["masking_top"]) == 10
    assert len(doc["references"]) == 3
    assert len(text) / 4 < 8000


# --- mix --plan (M2) -----------------------------------------------------------------------------


def _plan_project(root: Path, **overrides) -> Path:
    stems = [
        StemFixture("kick", "kick", fade(sine(60, amplitude=0.4, duration_s=2.0))),
        StemFixture("bass", "bass", fade(sine(110, amplitude=0.3, duration_s=2.0))),
        StemFixture("gtr", "electric_guitar", noise(amplitude=0.05, channels=2, duration_s=2.0)),
        StemFixture("lead_vox", "lead_vocal", pink_noise(rms=0.05, duration_s=2.0, seed=3)),
    ]
    return make_project(root, stems, **overrides)


PLAN_IDS = ["kick", "bass", "gtr", "lead_vox"]
RUN_FILES = [
    "analysis.json",
    "baseline.wav",
    "baseline_evaluation.json",
    "best.json",
    "iter_00",
    "report.md",
]


def _iterations(root: Path) -> list[Path]:
    return sorted((root / "runs").glob("*/iter_00"))


def _mix_plan(root: Path, plan: Path, *extra: str):
    return runner.invoke(app, ["mix", str(root), "--plan", str(plan), *extra])


@pytest.mark.req("REN-6")
def test_mix_with_plan_writes_iteration(tmp_path: Path) -> None:
    root = _plan_project(tmp_path / "song")
    inputs_before = _tree_digests(root / "stems")
    plan = write_plan(tmp_path / "plan.json", example_plan(PLAN_IDS))
    result = _mix_plan(root, plan)
    assert result.exit_code == 0, result.stderr
    assert "mix:" in result.stdout
    assert "pre-gain: kick" in result.stdout
    [iteration] = _iterations(root)
    assert sorted(p.name for p in iteration.parent.iterdir()) == RUN_FILES
    assert sorted(p.name for p in iteration.iterdir()) == [
        "evaluation.json",
        "mix.wav",
        "plan.json",
    ]
    info = sf.info(iteration / "mix.wav")
    assert (info.format, info.subtype, info.channels, info.samplerate) == ("WAV", "PCM_24", 2, SR)
    assert info.frames == sf.info(iteration.parent / "baseline.wav").frames == 2 * SR
    written = load_plan(iteration / "plan.json")
    assert written.iteration == 0  # the folder it lives in; the input file says 1 (D-026)
    assert written.model_copy(update={"iteration": 1}) == load_plan(plan)
    assert true_peak_db(read_audio(iteration / "mix.wav")[0]) <= -3.0
    assert _tree_digests(root / "stems") == inputs_before


@pytest.mark.req("REN-7")
def test_mix_with_plan_writes_baseline(tmp_path: Path) -> None:
    root = _plan_project(tmp_path / "song")
    plan = write_plan(tmp_path / "plan.json", example_plan(PLAN_IDS))
    assert _mix_plan(root, plan).exit_code == 0
    assert runner.invoke(app, ["mix", str(root), "--baseline-only"]).exit_code == 0
    first, second = _baselines(root)
    assert _digest(first) == _digest(second)


@pytest.mark.req("REN-6")
def test_mix_with_plan_exports_stems(tmp_path: Path) -> None:
    root = _plan_project(tmp_path / "song")
    plan = write_plan(tmp_path / "plan.json", example_plan(PLAN_IDS))
    result = _mix_plan(root, plan, "--export-stems")
    assert result.exit_code == 0, result.stderr
    [iteration] = _iterations(root)
    names = sorted(p.name for p in (iteration / "stems").iterdir())
    assert names == sorted(f"{sid}.wav" for sid in PLAN_IDS)
    assert (iteration / "reverb_return.wav").is_file()  # apart from stems/: no id can collide
    files = [*sorted((iteration / "stems").iterdir()), iteration / "reverb_return.wav"]
    tracks = [read_audio(p)[0].astype(np.float64) for p in files]
    mix = read_audio(iteration / "mix.wav")[0].astype(np.float64)
    lsb = 2.0**-23
    # One 24-bit write/read per file, ≤ 1 LSB each (libsndfile scales by 2^23 - 1 when writing and
    # by 2^23 when reading, see the M0 report): 5 tracks + the mix ⇒ ≤ 6 LSB (measured 4).
    assert np.max(np.abs(np.sum(tracks, axis=0) - mix)) <= (len(tracks) + 1) * lsb + 1e-12


def _set_stem(doc: dict, **values) -> dict:
    for sp in doc["stems"].values():
        sp.update(values)
    return doc


@pytest.mark.req("REN-6")
@pytest.mark.parametrize(
    ("content", "message"),
    [
        ("{not json", "not valid JSON"),
        (json.dumps(_set_stem(example_plan(PLAN_IDS), gain_db=20.0)), "gain_db"),
        (json.dumps(example_plan(PLAN_IDS[:3])), "missing from the plan: lead_vox"),
        (json.dumps(example_plan([*PLAN_IDS, "extra"])), "not in the project: extra"),
    ],
    ids=["not-json", "out-of-limits", "missing-stem", "unknown-stem"],
)
def test_mix_with_invalid_plan_fails(tmp_path: Path, content: str, message: str) -> None:
    root = _plan_project(tmp_path / "song")
    plan = tmp_path / "plan.json"
    plan.write_text(content, encoding="utf-8")
    result = _mix_plan(root, plan)
    assert result.exit_code == EXIT_ERROR
    assert message in result.stderr
    assert "Traceback" not in result.stderr
    assert not (root / "runs").exists()


@pytest.mark.req("REN-6")
def test_mix_with_plan_above_nyquist_fails(tmp_path: Path) -> None:
    stems = [
        StemFixture("kick", "kick", sine(60, sr=32_000, amplitude=0.3), sr=32_000),
        StemFixture("bass", "bass", sine(110, sr=32_000, amplitude=0.3), sr=32_000),
    ]
    root = make_project(tmp_path / "song", stems, sample_rate=32_000)
    plan = write_plan(
        tmp_path / "plan.json", _set_stem(example_plan(["kick", "bass"]), lpf_hz=16_000)
    )
    result = _mix_plan(root, plan)
    assert result.exit_code == EXIT_ERROR
    assert "Nyquist" in result.stderr
    assert not (root / "runs").exists()


def test_mix_option_conflicts(tmp_path: Path) -> None:
    root = _plan_project(tmp_path / "song")
    plan = write_plan(tmp_path / "plan.json", example_plan(PLAN_IDS))
    both = runner.invoke(app, ["mix", str(root), "--plan", str(plan), "--baseline-only"])
    assert both.exit_code == EXIT_ERROR
    assert "cannot be used together" in both.stderr
    export = runner.invoke(app, ["mix", str(root), "--baseline-only", "--export-stems"])
    assert export.exit_code == EXIT_ERROR
    assert "--export-stems cannot be used with --baseline-only" in export.stderr
    missing = _mix_plan(root, tmp_path / "nope.json")
    assert missing.exit_code == EXIT_ERROR
    assert "mix plan not found" in missing.stderr
    assert not (root / "runs").exists()


@pytest.mark.req("REN-2", "REN-4")
@pytest.mark.acceptance("M2")
def test_acceptance_same_plan_same_hash(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """Two runs of the same plan (EQ, compressor and reverb on) ⇒ identical mix.wav."""
    root = _plan_project(tmp_path / "song")
    monkeypatch.setattr(runs, "now", lambda: datetime(2026, 11, 10, 12, 0, 0))
    plan = write_plan(tmp_path / "plan.json", example_plan(PLAN_IDS))
    assert _mix_plan(root, plan, "--export-stems").exit_code == 0
    assert _mix_plan(root, plan, "--export-stems").exit_code == 0
    first, second = _iterations(root)
    assert first.parent.name != second.parent.name  # two runs, nothing overwritten
    assert _tree_digests(first) == _tree_digests(second)
    assert _digest(first / "mix.wav") == _digest(second / "mix.wav")


@pytest.mark.req("REN-6")
def test_export_warns_when_a_track_clips(tmp_path: Path) -> None:
    """Two stems in opposite phase: the mix cancels, but each exported stem exceeds 0 dBFS."""
    tone = fade(sine(110, amplitude=0.5, duration_s=1.0))
    stems = [StemFixture("a", "synth", tone), StemFixture("b", "synth", -tone)]
    root = make_project(tmp_path / "song", stems, options={"pre_normalize": False})
    plan = write_plan(tmp_path / "plan.json", neutral_plan(["a", "b"], gain_db=12.0))
    result = _mix_plan(root, plan, "--export-stems")
    assert result.exit_code == 0, result.stderr
    assert "exported track 'a' peaks above 0 dBFS" in result.stderr
    assert "exported track 'b' peaks above 0 dBFS" in result.stderr


# --- mix with a decider (M3) ---------------------------------------------------------------------

DECISION_IDS = ["kick", "bass", "gtr", "lead_vox"]  # ids of the default synthetic project


@pytest.fixture
def llm_env(tmp_path: Path, monkeypatch: pytest.MonkeyPatch):
    """Configuration of the real decider from the environment, a fake OpenRouter, no `.env`."""
    from stemwise import cli
    from stemwise.decision.llm import LlmDecider
    from tests.helpers.openrouter import KEY, MODEL, FakeOpenRouter

    monkeypatch.chdir(tmp_path)  # never read the author's .env from the repository
    for name in ("STEMWISE_TEMPERATURE", "STEMWISE_BASE_URL", "STEMWISE_REASONING_EFFORT"):
        monkeypatch.delenv(name, raising=False)
    monkeypatch.setenv("OPENROUTER_API_KEY", KEY)
    monkeypatch.setenv("STEMWISE_MODEL", MODEL)
    fake = FakeOpenRouter([])
    monkeypatch.setattr(
        cli, "_llm_decider", lambda cfg: LlmDecider(cfg, transport=fake.transport())
    )
    return fake


def _decision_runs(root: Path) -> list[Path]:
    return sorted(p for p in (root / "runs").iterdir() if p.is_dir())


@pytest.mark.req("DEC-7", "DEC-2")
@pytest.mark.acceptance("M3")
def test_mix_with_mock_decider(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """The complete flow runs offline with the mock decider, deterministically."""
    root = make_project(tmp_path / "song")
    monkeypatch.setattr(runs, "now", lambda: datetime(2026, 11, 10, 15, 0, 0))
    first = runner.invoke(app, ["mix", str(root), "--decider", "mock"])
    assert first.exit_code == 0, first.stderr
    assert "  decider: mock, no model call" in first.stdout
    run_dir = _decision_runs(root)[0]
    assert sorted(p.name for p in run_dir.iterdir()) == RUN_FILES  # no target: one iteration
    it = run_dir / "iter_00"
    assert sorted(p.name for p in it.iterdir()) == [
        "evaluation.json",
        "llm_request.json",
        "mix.wav",
        "plan.json",
    ]
    plan = load_plan(it / "plan.json")
    assert plan.iteration == 0
    assert list(plan.stems) == DECISION_IDS
    mix, fs = read_audio(it / "mix.wav")
    assert fs == SR
    assert true_peak_db(mix) <= -3.0
    request = json.loads((it / "llm_request.json").read_text(encoding="utf-8"))
    assert request["decider"] == "mock"
    assert request["sent"] is False
    assert [m["role"] for m in request["messages"]] == ["system", "user"]
    second = runner.invoke(app, ["mix", str(root), "--decider", "mock"])
    assert second.exit_code == 0, second.stderr
    a, b = (r / "iter_00" for r in _decision_runs(root))
    assert a != b
    assert _digest(a / "mix.wav") == _digest(b / "mix.wav")
    assert (a / "plan.json").read_bytes() == (b / "plan.json").read_bytes()


@pytest.mark.req("DEC-4", "DEC-8")
@pytest.mark.acceptance("M3")
def test_mix_llm_invalid_plans_fail_clearly(tmp_path: Path, llm_env) -> None:
    root = make_project(tmp_path / "song")
    bad = example_plan(DECISION_IDS)
    bad["stems"]["gtr"]["gain_db"] = 30.0
    llm_env.answers = [json.dumps(bad)] * 3
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code == EXIT_ERROR
    assert "no valid plan in 3 attempts" in result.stderr
    assert "stems.gtr.gain_db" in result.stderr
    assert "run folder kept" in result.stderr
    assert "Traceback" not in result.stderr
    assert len(llm_env.chat_requests) == 3
    (run_dir,) = _decision_runs(root)
    assert sorted(p.name for p in (run_dir / "iter_00").iterdir()) == [
        "llm_request.json",
        "llm_response.json",
    ]
    assert (run_dir / "analysis.json").is_file()
    assert (run_dir / "baseline.wav").is_file()
    response = json.loads((run_dir / "iter_00" / "llm_response.json").read_text(encoding="utf-8"))
    assert [a["valid"] for a in response["attempts"]] == [False, False, False]


@pytest.mark.req("DEC-4")
@pytest.mark.acceptance("M3")
def test_mix_llm_recovers_from_non_json(tmp_path: Path, llm_env) -> None:
    root = make_project(tmp_path / "song")
    llm_env.answers = ["Of course! Here is my mix.", json.dumps(example_plan(DECISION_IDS))]
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code == 0, result.stderr
    assert "  decider: llm (vendor/test-model), 2 attempt(s)" in result.stdout
    assert "cost 0.0042 USD" in result.stdout
    it = _decision_runs(root)[0] / "iter_00"
    assert sorted(p.name for p in it.iterdir()) == [
        "evaluation.json",
        "llm_request.json",
        "llm_response.json",
        "mix.wav",
        "plan.json",
    ]
    assert load_plan(it / "plan.json").summary == example_plan(DECISION_IDS)["summary"]


@pytest.mark.req("DEC-6")
def test_mix_llm_without_configuration_fails_before_work(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, llm_env
) -> None:
    root = make_project(tmp_path / "song")
    monkeypatch.delenv("STEMWISE_MODEL")
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code == EXIT_ERROR
    assert "STEMWISE_MODEL" in result.stderr
    assert not (root / "runs").exists()
    assert llm_env.requests == []


@pytest.mark.req("DEC-6")
def test_mix_llm_options_override_environment(tmp_path: Path, llm_env) -> None:
    root = make_project(tmp_path / "song")
    llm_env.model = "vendor/other-model"
    llm_env.answers = [json.dumps(example_plan(DECISION_IDS))]
    args = ["--model", "vendor/other-model", "--temperature", "0.5", "--reasoning-effort", "high"]
    result = runner.invoke(app, ["mix", str(root), *args])
    assert result.exit_code == 0, result.stderr
    body = llm_env.chat_bodies()[0]
    assert (body["model"], body["temperature"], body["reasoning"]) == (
        "vendor/other-model",
        0.5,
        {"effort": "high"},
    )


def test_mix_llm_model_unavailable_leaves_no_run(tmp_path: Path, llm_env) -> None:
    root = make_project(tmp_path / "song")
    llm_env.model = "vendor/not-the-configured-one"
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code == EXIT_ERROR
    assert "is not available" in result.stderr
    assert not (root / "runs").exists()  # checked before the analysis (D-044): no run at all
    assert llm_env.chat_requests == []


@pytest.mark.req("DEC-8")
def test_mix_llm_never_prints_or_writes_the_key(tmp_path: Path, llm_env) -> None:
    from tests.helpers.openrouter import KEY

    root = make_project(tmp_path / "song")
    llm_env.answers = ["nope", json.dumps(example_plan(DECISION_IDS))]
    result = runner.invoke(app, ["mix", str(root), "--export-stems"])
    assert result.exit_code == 0, result.stderr
    assert KEY not in result.stdout + result.stderr
    for path in (root / "runs").rglob("*"):
        if path.is_file():
            assert KEY.encode() not in path.read_bytes(), path


@pytest.mark.parametrize(
    "extra",
    [["--decider", "mock"], ["--model", "vendor/x"], ["--temperature", "0.3"]],
)
def test_decider_options_rejected_without_decision(tmp_path: Path, extra: list[str]) -> None:
    root = make_project(tmp_path / "song")
    plan = write_plan(tmp_path / "plan.json", neutral_plan(DECISION_IDS))
    for mode in (["--baseline-only"], ["--plan", str(plan)]):
        result = runner.invoke(app, ["mix", str(root), *mode, *extra])
        assert result.exit_code == EXIT_ERROR
        assert "cannot be used with" in result.stderr
    assert not (root / "runs").exists()


def test_interrupted_decision_keeps_the_paid_logs(tmp_path: Path, llm_env) -> None:
    """D-040: Ctrl-C during the second call still leaves the logs of the first one."""
    root = make_project(tmp_path / "song")

    def interrupt(request):
        raise KeyboardInterrupt

    llm_env.answers = ["not a plan", interrupt]
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code != 0
    (run_dir,) = _decision_runs(root)
    request = json.loads((run_dir / "iter_00" / "llm_request.json").read_text(encoding="utf-8"))
    assert len(request["attempts"]) == 2
    response = json.loads((run_dir / "iter_00" / "llm_response.json").read_text(encoding="utf-8"))
    assert response["attempts"][0]["valid"] is False


# --- the verification loop (M4) ------------------------------------------------------------------


def _loop_project(root: Path, **overrides) -> Path:
    reference = pink_noise(duration_s=4.0, channels=2, rms=0.1, seed=9)
    return make_project(root, references=[("ref.wav", reference, SR)], **overrides)


def _iteration_dirs(run_dir: Path) -> list[Path]:
    return sorted(run_dir.glob("iter_*"))


@pytest.mark.req("EVA-5")
def test_max_iter_option(tmp_path: Path) -> None:
    root = _loop_project(tmp_path / "song")
    result = runner.invoke(app, ["mix", str(root), "--decider", "mock", "--max-iter", "2"])
    assert result.exit_code == 0, result.stderr
    (run_dir,) = _decision_runs(root)
    assert [p.name for p in _iteration_dirs(run_dir)] == ["iter_00", "iter_01"]
    assert "best: iter_01/plan.json (stop: max_iterations)" in result.stdout
    for bad in ("0", "4"):
        usage = runner.invoke(app, ["mix", str(root), "--decider", "mock", "--max-iter", bad])
        assert usage.exit_code == 2  # usage error of the CLI parser
    plan = write_plan(tmp_path / "plan.json", neutral_plan(DECISION_IDS))
    both = runner.invoke(app, ["mix", str(root), "--plan", str(plan), "--max-iter", "2"])
    assert both.exit_code == EXIT_ERROR
    assert "--max-iter cannot be used with --plan" in both.stderr
    assert len(_decision_runs(root)) == 1


@pytest.mark.req("EVA-4", "EVA-6")
def test_no_valid_iteration_exits_with_an_error(tmp_path: Path, llm_env) -> None:
    """A stem with 1% DC and a plan without high-pass: the only iteration breaks EVA-4."""
    stems = [
        StemFixture("a", "synth", fade(sine(220, amplitude=0.2, duration_s=2.0)) + 0.01),
        StemFixture("b", "keys", fade(sine(440, amplitude=0.2, duration_s=2.0))),
    ]
    root = make_project(tmp_path / "song", stems)
    llm_env.answers = [json.dumps(neutral_plan(["a", "b"]))]
    result = runner.invoke(app, ["mix", str(root), "--max-iter", "1"])
    assert result.exit_code == EXIT_ERROR
    assert "violates: DC offset" in result.stdout
    assert "no iteration satisfies the technical constraints" in result.stderr
    (run_dir,) = _decision_runs(root)
    best = json.loads((run_dir / "best.json").read_text(encoding="utf-8"))
    assert best == {
        "iteration": None,
        "plan": None,
        "spectral_distance_db": None,
        "stop_reason": "max_iterations",
    }


@pytest.mark.req("EVA-5", "EVA-6")
def test_later_decision_failure_exits_zero_with_a_warning(tmp_path: Path, llm_env) -> None:
    root = _loop_project(tmp_path / "song")
    llm_env.answers = [json.dumps(example_plan(DECISION_IDS)), "nope", "nope", "nope"]
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code == 0, result.stderr
    assert "the decision of iteration 1 failed" in result.stderr
    assert "best: iter_00/plan.json (stop: decision_failed)" in result.stdout


def test_interrupt_in_a_later_iteration_keeps_the_logs(tmp_path: Path, llm_env) -> None:
    root = _loop_project(tmp_path / "song")

    def interrupt(request):
        raise KeyboardInterrupt

    llm_env.answers = [json.dumps(example_plan(DECISION_IDS)), "nope", interrupt]
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code != 0
    (run_dir,) = _decision_runs(root)
    assert (run_dir / "iter_00" / "evaluation.json").is_file()
    assert json.loads((run_dir / "best.json").read_text(encoding="utf-8"))["iteration"] == 0
    request = json.loads((run_dir / "iter_01" / "llm_request.json").read_text(encoding="utf-8"))
    assert len(request["attempts"]) == 2


@pytest.mark.req("EVA-5")
@pytest.mark.acceptance("M4")
def test_loop_is_deterministic(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    root = _loop_project(tmp_path / "song")
    monkeypatch.setattr(runs, "now", lambda: datetime(2026, 11, 10, 18, 0, 0))
    for _ in range(2):
        result = runner.invoke(app, ["mix", str(root), "--decider", "mock"])
        assert result.exit_code == 0, result.stderr
    first, second = _decision_runs(root)
    a, b = _iteration_dirs(first), _iteration_dirs(second)
    assert len(a) == len(b) == 3  # the mock moves towards the target at every iteration
    for x, y in zip(a, b, strict=True):
        assert _digest(x / "mix.wav") == _digest(y / "mix.wav")
        assert (x / "evaluation.json").read_bytes() == (y / "evaluation.json").read_bytes()
    assert (first / "best.json").read_bytes() == (second / "best.json").read_bytes()


def _table(report: str) -> dict[str, list[str]]:
    section = report.split("## Iterations", 1)[1].split("## Summaries", 1)[0]
    rows = {}
    for line in section.splitlines():
        if line.startswith("| ") and not line.startswith("| ---"):
            cells = [c.strip() for c in line.strip("|").split("|")]
            rows[cells[0].strip("*").replace(" (best)", "")] = cells[1:]
    rows.pop("", None)
    return rows


@pytest.mark.req("REP-1", "EVA-1")
@pytest.mark.acceptance("M4")
def test_report_compares_baseline_with_every_iteration(tmp_path: Path) -> None:
    root = _loop_project(tmp_path / "song")
    result = runner.invoke(app, ["mix", str(root), "--decider", "mock"])
    assert result.exit_code == 0, result.stderr
    (run_dir,) = _decision_runs(root)
    assert f"report: {run_dir / 'report.md'}" in result.stdout
    rows = _table((run_dir / "report.md").read_text(encoding="utf-8"))
    iterations = _iteration_dirs(run_dir)
    assert list(rows) == ["raw sum", *(p.name for p in iterations)]
    files = {"raw sum": run_dir / "baseline_evaluation.json"}
    files |= {p.name: p / "evaluation.json" for p in iterations}
    for label, path in files.items():
        distance = json.loads(path.read_text(encoding="utf-8"))["spectral_distance_db"]
        assert rows[label][0] == f"{distance:.1f}"


@pytest.mark.req("EVA-6")
@pytest.mark.acceptance("M4")
def test_best_plan_has_min_distance_among_valid(tmp_path: Path, llm_env) -> None:
    """Three scripted plans: the second is closer to the reference, the third goes back."""
    first = example_plan(DECISION_IDS)
    closer = example_plan(DECISION_IDS)
    closer["stems"]["gtr"]["gain_db"] += 4.0
    llm_env.answers = [json.dumps(first), json.dumps(closer), json.dumps(first)]
    root = _loop_project(tmp_path / "song")
    result = runner.invoke(app, ["mix", str(root)])
    assert result.exit_code == 0, result.stderr
    (run_dir,) = _decision_runs(root)
    evaluations = [
        json.loads((p / "evaluation.json").read_text(encoding="utf-8"))
        for p in _iteration_dirs(run_dir)
    ]
    assert len(evaluations) == 3
    valid = [e for e in evaluations if all(e["constraints"].values())]
    best = json.loads((run_dir / "best.json").read_text(encoding="utf-8"))
    assert best["iteration"] == 1
    assert best["plan"] == "iter_01/plan.json"
    assert best["spectral_distance_db"] == min(e["spectral_distance_db"] for e in valid)
    assert [e["is_best"] for e in evaluations] == [True, True, False]
    assert evaluations[2]["spectral_distance_db"] > best["spectral_distance_db"]
    assert "**iter_01 (best)**" in (run_dir / "report.md").read_text(encoding="utf-8")


@pytest.mark.req("REP-1")
@pytest.mark.acceptance("M4")
def test_report_command(tmp_path: Path) -> None:
    root = _loop_project(tmp_path / "song")
    assert (
        runner.invoke(app, ["mix", str(root), "--decider", "mock", "--max-iter", "1"]).exit_code
        == 0
    )
    assert runner.invoke(app, ["mix", str(root), "--decider", "mock"]).exit_code == 0
    older, latest = _decision_runs(root)
    original = (latest / "report.md").read_bytes()
    (latest / "report.md").unlink()
    result = runner.invoke(app, ["report", str(root)])
    assert result.exit_code == 0, result.stderr
    assert f"report: {latest / 'report.md'}" in result.stdout
    assert (latest / "report.md").read_bytes() == original
    by_name = runner.invoke(app, ["report", str(root), "--run", older.name])
    assert by_name.exit_code == 0, by_name.stderr
    assert str(older / "report.md") in by_name.stdout
    missing = runner.invoke(app, ["report", str(root), "--run", "nope"])
    assert missing.exit_code == EXIT_ERROR
    assert "run folder not found" in missing.stderr
    analysis_only = make_project(tmp_path / "other")
    assert runner.invoke(app, ["analyze", str(analysis_only)]).exit_code == 0
    refused = runner.invoke(app, ["report", str(analysis_only)])
    assert refused.exit_code == EXIT_ERROR
    assert "run `stemwise mix` first" in refused.stderr


@pytest.mark.req("REP-1")
def test_mix_with_plan_writes_a_report(tmp_path: Path) -> None:
    root = _loop_project(tmp_path / "song")
    plan = write_plan(tmp_path / "plan.json", example_plan(DECISION_IDS))
    result = _mix_plan(root, plan)
    assert result.exit_code == 0, result.stderr
    (run_dir,) = _decision_runs(root)
    report = (run_dir / "report.md").read_text(encoding="utf-8")
    assert "- Decider: hand-written plan (`mix --plan`)" in report
    assert list(_table(report)) == ["raw sum", "iter_00"]


# --- revise (M5, REV-1 → REV-3) -------------------------------------------------------------------


def _mixed(tmp_path: Path, *args: str) -> Path:
    root = _loop_project(tmp_path / "song")
    result = runner.invoke(app, ["mix", str(root), "--decider", "mock", *args])
    assert result.exit_code == 0, result.stderr
    return root


def _revise(root: Path, request: str, *args: str):
    return runner.invoke(app, ["revise", str(root), request, *args])


def _diff_rows(report: str) -> list[list[str]]:
    section = report.split("## Changes from the starting plan", 1)[1].split("\n## ", 1)[0]
    rows = [
        [c.strip() for c in line.strip("|").split("|")]
        for line in section.splitlines()
        if line.startswith("| ") and not line.startswith("| ---")
    ]
    return rows[1:]


@pytest.mark.req("REV-1", "REV-3")
@pytest.mark.acceptance("M5")
def test_revise_voice_forward_shows_a_diff(tmp_path: Path) -> None:
    """PRD §9 (M5): "voce più avanti" ⇒ a diff on the vocal or on the stems that cover it."""
    root = _mixed(tmp_path)
    (source,) = _decision_runs(root)
    result = _revise(root, "voce più avanti", "--decider", "mock")
    assert result.exit_code == 0, result.stderr
    revision = _decision_runs(root)[-1]
    assert revision != source
    assert f"revision: from run {source.name}" in result.stdout
    assert f"report: {revision / 'report.md'}" in result.stdout
    report = (revision / "report.md").read_text(encoding="utf-8")
    rows = _diff_rows(report)
    start = json.loads((revision / "source_plan.json").read_text(encoding="utf-8"))
    collisions = json.loads((revision / "source_evaluation.json").read_text(encoding="utf-8"))
    covering = {c["a"] if c["b"] == "lead_vox" else c["b"] for c in collisions["masking_top"]}
    covering &= set(DECISION_IDS) - {"lead_vox"}
    touched = {r[0] for r in rows}
    assert touched
    assert touched <= {"lead_vox", *covering}
    gain = start["stems"]["lead_vox"]["gain_db"]
    assert ["lead_vox", "gain_db", f"{gain:+.1f} dB", f"{gain + 2:+.1f} dB"] in rows


@pytest.mark.req("REV-1")
@pytest.mark.acceptance("M5")
def test_revise_is_deterministic(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    root = _mixed(tmp_path)
    (source,) = _decision_runs(root)
    monkeypatch.setattr(runs, "now", lambda: datetime(2026, 11, 10, 22, 0, 0))
    for _ in range(2):
        result = _revise(root, "voce più avanti", "--decider", "mock", "--run", source.name)
        assert result.exit_code == 0, result.stderr
    _, a, b = _decision_runs(root)
    assert _digest(a / "iter_00" / "mix.wav") == _digest(b / "iter_00" / "mix.wav")
    assert (a / "iter_00" / "plan.json").read_bytes() == (b / "iter_00" / "plan.json").read_bytes()


@pytest.mark.req("REV-1")
def test_revise_leaves_the_source_run_untouched(tmp_path: Path) -> None:
    root = _mixed(tmp_path)
    (source,) = _decision_runs(root)
    before = _tree_digests(source)
    inputs = {d: _tree_digests(root / d) for d in ("stems", "references")}
    assert _revise(root, "voce più avanti", "--decider", "mock").exit_code == 0
    assert _tree_digests(source) == before
    assert {d: _tree_digests(root / d) for d in ("stems", "references")} == inputs


@pytest.mark.req("REV-2")
def test_revise_chains_on_the_latest_revision(tmp_path: Path) -> None:
    root = _mixed(tmp_path)
    assert _revise(root, "voce più avanti", "--decider", "mock").exit_code == 0
    first = _decision_runs(root)[-1]
    assert _revise(root, "voce più avanti", "--decider", "mock").exit_code == 0
    second = _decision_runs(root)[-1]
    revision = json.loads((second / "revision.json").read_text(encoding="utf-8"))
    assert revision["source_run"] == first.name
    assert revision["source_plan"] == "iter_00/plan.json"
    gains = [
        json.loads((r / "iter_00" / "plan.json").read_text(encoding="utf-8"))["stems"]["lead_vox"][
            "gain_db"
        ]
        for r in (first, second)
    ]
    assert gains[1] == gains[0] + 2.0


@pytest.mark.req("REV-1")
@pytest.mark.parametrize(
    ("args", "code", "message"),
    [
        (["   "], 2, "request"),
        (["x" * 1001], 2, "1000 characters"),
        (["louder", "--max-iter", "2"], 2, "--max-iter"),
        (["louder", "--run", "nope"], EXIT_ERROR, "run folder not found"),
    ],
)
def test_revise_options(tmp_path: Path, args: list[str], code: int, message: str) -> None:
    root = _mixed(tmp_path, "--max-iter", "1")
    result = runner.invoke(app, ["revise", str(root), *args, "--decider", "mock"])
    assert result.exit_code == code
    assert message in result.stderr
    assert len(_decision_runs(root)) == 1  # no new run


@pytest.mark.req("REV-1")
def test_revise_without_a_mix_run(tmp_path: Path) -> None:
    root = _loop_project(tmp_path / "song")
    result = _revise(root, "louder", "--decider", "mock")
    assert result.exit_code == EXIT_ERROR
    assert "run `stemwise mix` first" in result.stderr


@pytest.mark.req("REV-1")
def test_revise_llm_without_configuration_fails_before_work(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, llm_env
) -> None:
    root = _mixed(tmp_path, "--max-iter", "1")
    monkeypatch.delenv("STEMWISE_MODEL")
    result = _revise(root, "louder")
    assert result.exit_code == EXIT_ERROR
    assert "STEMWISE_MODEL" in result.stderr
    assert len(_decision_runs(root)) == 1
    assert llm_env.requests == []


@pytest.mark.req("REV-1")
def test_revise_failed_decision_keeps_logs(tmp_path: Path, llm_env) -> None:
    root = _mixed(tmp_path, "--max-iter", "1")
    llm_env.answers = ["nope"] * 3
    result = _revise(root, "voce più avanti")
    assert result.exit_code == EXIT_ERROR
    assert "run folder kept" in result.stderr
    revision = _decision_runs(root)[-1]
    assert (revision / "revision.json").is_file()
    assert sorted(p.name for p in (revision / "iter_00").iterdir()) == [
        "llm_request.json",
        "llm_response.json",
    ]
    user = llm_env.chat_bodies()[0]["messages"][1]["content"]
    assert "## Starting plan (the mix the author listened to)" in user
    assert "## Author's request (top priority)\n\nvoce più avanti" in user


@pytest.mark.req("REV-2")
def test_revise_violating_constraints_exits_1(tmp_path: Path, llm_env) -> None:
    """The starting plan high-passes a stem with 1% DC; the revised plan drops the filter."""
    stems = [
        StemFixture("a", "synth", fade(sine(220, amplitude=0.2, duration_s=2.0)) + 0.01),
        StemFixture("b", "keys", fade(sine(440, amplitude=0.2, duration_s=2.0))),
    ]
    root = make_project(tmp_path / "song", stems)
    start = neutral_plan(["a", "b"])
    start["stems"]["a"]["hpf_hz"] = 20.0
    assert _mix_plan(root, write_plan(tmp_path / "start.json", start)).exit_code == 0
    llm_env.answers = [json.dumps(neutral_plan(["a", "b"]))]
    result = _revise(root, "no filters")
    assert result.exit_code == EXIT_ERROR
    assert "violates: DC offset" in result.stdout
    assert "breaks a technical constraint" in result.stderr
    assert "--run" in result.stderr
    revision = _decision_runs(root)[-1]
    assert json.loads((revision / "best.json").read_text(encoding="utf-8"))["iteration"] is None
    assert (revision / "iter_00" / "mix.wav").is_file()
    assert "Result: none within the technical constraints" in (revision / "report.md").read_text(
        encoding="utf-8"
    )


@pytest.mark.req("REV-3")
def test_report_regenerates_a_revision(tmp_path: Path) -> None:
    root = _mixed(tmp_path, "--max-iter", "1")
    assert _revise(root, "voce più avanti", "--decider", "mock").exit_code == 0
    revision = _decision_runs(root)[-1]
    original = (revision / "report.md").read_bytes()
    (revision / "report.md").unlink()
    result = runner.invoke(app, ["report", str(root)])
    assert result.exit_code == 0, result.stderr
    assert (revision / "report.md").read_bytes() == original
    assert original.startswith(b"# Revision report")


# --- master (M6) ---------------------------------------------------------------------------------


def _master(root: Path, *args: str):
    return runner.invoke(app, ["master", str(root), *args])


def _report_doc(result_stdout: str) -> dict:
    line = next(x for x in result_stdout.splitlines() if x.startswith("report: "))
    return json.loads(Path(line.removeprefix("report: ")).read_text(encoding="utf-8"))


@pytest.mark.req("MAS-1", "MAS-2", "MAS-3")
@pytest.mark.acceptance("M6")
def test_mix_then_master(tmp_path: Path) -> None:
    """PRD §9 (M6): master.wav produced; master_report.json has LUFS, true peak and distance."""
    from stemwise.master import MasterReport

    root = _mixed(tmp_path)
    (run_dir,) = _decision_runs(root)
    result = _master(root)
    assert result.exit_code == 0, result.stderr
    master_wav = run_dir / "master" / "master.wav"
    info = sf.info(str(master_wav))
    assert (info.format, info.subtype, info.samplerate, info.channels) == ("WAV", "PCM_24", SR, 2)
    doc = _report_doc(result.stdout)
    report = MasterReport.model_validate(doc)
    assert report.lufs is not None
    assert report.true_peak_db is not None
    assert report.spectral_distance_db is not None
    assert f"master: {master_wav} ({SR} Hz)" in result.stdout
    assert f"mix: run {run_dir.name}, iter_" in result.stdout
    assert "reference: ref1 " in result.stdout
    assert "distance from the reference: mix " in result.stdout


@pytest.mark.acceptance("M6")
@pytest.mark.req("MAS-1")
def test_full_flow_offline(tmp_path: Path) -> None:
    """PRD §10.1: analyze, mix, revise and master with the mock decider on a synthetic project."""
    root = _loop_project(tmp_path / "song")
    assert runner.invoke(app, ["analyze", str(root)]).exit_code == 0
    assert runner.invoke(app, ["mix", str(root), "--decider", "mock"]).exit_code == 0
    revised = _revise(root, "voce più avanti", "--decider", "mock")
    assert revised.exit_code == 0, revised.stderr
    revision = _decision_runs(root)[-1]
    result = _master(root)
    assert result.exit_code == 0, result.stderr
    assert (revision / "master" / "master.wav").is_file()
    assert _report_doc(result.stdout)["mix"]["run"] == revision.name  # the latest run


@pytest.mark.req("MAS-1")
@pytest.mark.parametrize(
    ("setup", "args", "code", "message"),
    [
        ("mixed", ["--run", "x", "--mix", "m.wav"], EXIT_ERROR, "cannot be used together"),
        ("mixed", ["--reference", "ref5"], EXIT_ERROR, "has 1 references"),
        ("mixed", ["--reference", "nowhere.wav"], EXIT_ERROR, "reference file not found"),
        ("mixed", ["--run", "no-such-run"], EXIT_ERROR, "run folder not found"),
        ("unmixed", [], EXIT_ERROR, "--mix"),
        ("no_reference", ["--mix", "MIX"], EXIT_ERROR, "needs a reference"),
        ("mixed", ["--no-such-option"], 2, ""),
    ],
)
def test_master_options(
    tmp_path: Path, setup: str, args: list[str], code: int, message: str
) -> None:
    if setup == "mixed":
        root = _mixed(tmp_path)
    elif setup == "unmixed":
        root = _loop_project(tmp_path / "song")
    else:
        root = make_project(tmp_path / "song")
    args = [
        str(write_audio(tmp_path / "m.wav", pink_noise(duration_s=2.0, channels=2)))
        if a == "MIX"
        else a
        for a in args
    ]
    result = _master(root, *args)
    assert result.exit_code == code
    if message:
        assert message in result.stderr


@pytest.mark.req("MAS-3")
def test_master_true_peak_warning_reaches_the_cli(tmp_path: Path) -> None:
    """D-074: a loud reference gives a true peak above −1 dBTP: warning, exit 0, file kept."""
    loud = pink_noise(duration_s=4.0, channels=2, rms=0.2, seed=9)
    root = make_project(tmp_path / "song", references=[("ref.wav", loud, SR)])
    assert runner.invoke(app, ["mix", str(root), "--decider", "mock"]).exit_code == 0
    result = _master(root, "--pcm16")
    assert result.exit_code == 0, result.stderr
    assert "warning: true peak of the master is" in result.stderr
    assert "(above -1.0 dBTP)" in result.stdout
    assert "master_16.wav" in result.stdout
