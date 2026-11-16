"""Run folders: never overwrite a previous run (PRD §8); analysis run (D-019); decision run (M3)."""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path

import pytest

from stemwise.analysis.model import Analysis
from stemwise.decision.mock import MockDecider
from stemwise.runs import analysis_run, create_run_dir, mix_run
from tests.helpers.project import make_project

NOW = datetime(2026, 11, 9, 21, 30, 0)


def test_run_dir_named_after_local_time(tmp_path: Path) -> None:
    run = create_run_dir(tmp_path, NOW)
    assert run == tmp_path / "runs" / "2026-11-09T21-30-00"
    assert run.is_dir()


def test_run_dir_collision_gets_suffix(tmp_path: Path) -> None:
    first = create_run_dir(tmp_path, NOW)
    (first / "marker").write_text("first", encoding="utf-8")
    second = create_run_dir(tmp_path, NOW)
    third = create_run_dir(tmp_path, NOW)
    assert [second.name, third.name] == ["2026-11-09T21-30-00-2", "2026-11-09T21-30-00-3"]
    assert (first / "marker").read_text(encoding="utf-8") == "first"


def test_analysis_run(tmp_path: Path) -> None:
    root = make_project(tmp_path / "song")
    first = analysis_run(root, NOW)
    second = analysis_run(root, NOW)
    assert first.analysis_file == root / "runs" / "2026-11-09T21-30-00" / "analysis.json"
    assert second.run_dir.name == "2026-11-09T21-30-00-2"
    assert first.analysis_file.read_text(encoding="utf-8") == first.text
    assert Analysis.model_validate(json.loads(first.text)) == first.analysis
    assert sorted(p.name for p in first.run_dir.iterdir()) == ["analysis.json"]


# --- decision run (M3, D-038, D-040) -------------------------------------------------------------


def _llm(fake):
    from stemwise.decision.config import LlmConfig
    from stemwise.decision.llm import LlmDecider
    from tests.helpers.openrouter import KEY, MODEL

    return LlmDecider(
        LlmConfig(model=MODEL, api_key=KEY, reasoning_effort="high"), transport=fake.transport()
    )


@pytest.mark.req("DEC-8")
def test_llm_run_writes_complete_logs(tmp_path: Path) -> None:
    from tests.helpers.openrouter import KEY, FakeOpenRouter, valid_plan_text

    root = make_project(tmp_path / "song")
    ids = ["kick", "bass", "gtr", "lead_vox"]
    fake = FakeOpenRouter(["not a plan", valid_plan_text(ids)], parameters=("structured_outputs",))
    run = mix_run(root, _llm(fake), when=NOW)
    it = run.run_dir / "iter_00"
    request = json.loads((it / "llm_request.json").read_text(encoding="utf-8"))
    response = json.loads((it / "llm_response.json").read_text(encoding="utf-8"))
    assert request["decider"] == "llm"
    assert "api_key" not in request["config"]
    assert [a["body"] for a in request["attempts"]] == fake.chat_bodies()
    assert [a["attempt"] for a in response["attempts"]] == [1, 2]
    assert [a["status"] for a in response["attempts"]] == [200, 200]
    assert all(a["duration_s"] >= 0 for a in response["attempts"])
    assert response["total"]["prompt_tokens"] == 2000
    assert response["total"]["cost_usd"] == pytest.approx(0.0042)
    # parameters the model does not take are reported, not sent (D-045)
    assert any("temperature is not supported" in w for w in request["warnings"])
    assert request["attempts"][0]["body"]["messages"][1]["content"].count(run.analysis_text.strip())
    for path in run.run_dir.rglob("*"):
        if path.is_file():
            assert KEY.encode() not in path.read_bytes()


@pytest.mark.req("DEC-5")
def test_run_imposes_iteration_on_the_plan(tmp_path: Path) -> None:
    from tests.helpers.openrouter import FakeOpenRouter
    from tests.helpers.plan import example_plan

    root = make_project(tmp_path / "song")
    doc = example_plan(["kick", "bass", "gtr", "lead_vox"])
    doc["iteration"] = 5
    run = mix_run(root, _llm(FakeOpenRouter([json.dumps(doc)])), when=NOW)
    assert json.loads(run.iterations[0].plan_file.read_text(encoding="utf-8"))["iteration"] == 0


def test_mock_run_removed_when_render_fails(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    from stemwise import runs

    root = make_project(tmp_path / "song")

    def failing_render(*args, **kwargs):
        raise OSError("disk full")

    monkeypatch.setattr(runs, "render_mix", failing_render)
    with pytest.raises(OSError, match="disk full"):
        mix_run(root, MockDecider(), when=NOW)
    assert list((root / "runs").iterdir()) == []  # no model was called: nothing to keep


# --- the verification loop (M4) ------------------------------------------------------------------


def _loop_project(root: Path, **overrides) -> Path:
    from tests.helpers.signals import pink_noise

    reference = pink_noise(duration_s=4.0, channels=2, rms=0.1, seed=9)
    return make_project(root, references=[("ref.wav", reference, 48_000)], **overrides)


def _read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


@pytest.mark.req("EVA-1")
def test_baseline_evaluation_matches_analysis_mix(tmp_path: Path) -> None:
    run = mix_run(_loop_project(tmp_path / "song"), MockDecider(), when=NOW)
    analysis = _read(run.analysis_file)
    baseline = _read(run.run_dir / "baseline_evaluation.json")
    assert baseline["mix"] == analysis["mix"]
    assert baseline["iteration"] is None
    assert baseline["spectral_distance_db"] is not None  # a reference gives a target


@pytest.mark.req("EVA-5", "EVA-6")
def test_loop_files_are_consistent(tmp_path: Path) -> None:
    run = mix_run(_loop_project(tmp_path / "song"), MockDecider(), when=NOW)
    evaluations = [_read(it.evaluation_file) for it in run.iterations]
    assert [e["iteration"] for e in evaluations] == list(range(len(run.iterations)))
    assert all(e["stop_reason"] is None for e in evaluations[:-1])
    assert evaluations[-1]["stop_reason"] == run.best.stop_reason
    best = _read(run.best_file)
    assert best == run.best.model_dump(mode="json")
    valid = [e for e in evaluations if all(e["constraints"].values())]
    assert best["spectral_distance_db"] == min(e["spectral_distance_db"] for e in valid)
    assert (run.run_dir / best["plan"]).is_file()
    for it in run.iterations:
        assert sorted(p.name for p in it.dir.iterdir()) == [
            "evaluation.json",
            "llm_request.json",
            "mix.wav",
            "plan.json",
        ]


@pytest.mark.req("EVA-5")
def test_max_iterations_option_and_project(tmp_path: Path) -> None:
    root = _loop_project(tmp_path / "song", options={"max_iterations": 1})
    assert len(mix_run(root, MockDecider(), when=NOW).iterations) == 1
    one = mix_run(_loop_project(tmp_path / "other"), MockDecider(), max_iterations=1, when=NOW)
    assert len(one.iterations) == 1
    assert one.best.stop_reason in ("max_iterations", "below_threshold")


@pytest.mark.req("EVA-6")
def test_later_decision_failure_keeps_the_run(tmp_path: Path) -> None:
    from tests.helpers.openrouter import FakeOpenRouter, valid_plan_text

    ids = ["kick", "bass", "gtr", "lead_vox"]
    fake = FakeOpenRouter([valid_plan_text(ids), "nope", "nope", "nope"])
    run = mix_run(_loop_project(tmp_path / "song"), _llm(fake), when=NOW)
    assert len(run.iterations) == 1
    assert run.failed_iteration == 1
    assert run.best.iteration == 0
    assert run.best.stop_reason == "decision_failed"
    failed = run.run_dir / "iter_01"
    assert sorted(p.name for p in failed.iterdir()) == ["llm_request.json", "llm_response.json"]
    assert len(_read(failed / "llm_response.json")["attempts"]) == 3
    assert _read(run.best_file)["stop_reason"] == "decision_failed"


@pytest.mark.req("DEC-1", "DEC-5")
def test_second_call_carries_the_history(tmp_path: Path) -> None:
    from tests.helpers.openrouter import FakeOpenRouter
    from tests.helpers.plan import example_plan

    ids = ["kick", "bass", "gtr", "lead_vox"]
    first = example_plan(ids)
    jump = example_plan(ids)
    for sp in jump["stems"].values():
        sp["gain_db"] = 10.0  # +8 dB from the first plan: clipped to +6 (DEC-5)
    fake = FakeOpenRouter([json.dumps(first), json.dumps(jump), json.dumps(first)])
    run = mix_run(_loop_project(tmp_path / "song"), _llm(fake), max_iterations=2, when=NOW)
    assert len(run.iterations) == len(fake.chat_requests) == 2  # deterministic synthetic project
    user = fake.chat_bodies()[1]["messages"][1]["content"]
    assert "## History (last 1 iterations, oldest first)" in user
    assert '"spectral_distance_db"' in user
    second = run.iterations[1]
    assert second.decision.plan.stems["gtr"].gain_db == 6.0
    request = _read(second.dir / "llm_request.json")
    assert any("gain_db" in w for w in request["warnings"])


# --- revision run (M5, REV-1, REV-2, D-059 → D-063) -----------------------------------------------


class Recording:
    """Wraps a decider and records the inputs it receives."""

    def __init__(self, inner) -> None:
        self.inner = inner
        self.name = inner.name
        self.inputs = []

    def decide(self, decision_input):
        self.inputs.append(decision_input)
        return self.inner.decide(decision_input)


class Returns:
    """A decider that returns one fixed plan document, with a log (so warnings are written)."""

    name = "scripted"

    def __init__(self, doc: dict) -> None:
        self.doc = doc

    def decide(self, decision_input):
        from stemwise.decision.base import Decision
        from stemwise.decision.schema import MixPlan

        return Decision(plan=MixPlan.model_validate(self.doc), request_log={"decider": self.name})


def _hashes(folder: Path) -> dict[str, str]:
    import hashlib

    return {
        str(p.relative_to(folder)): hashlib.sha256(p.read_bytes()).hexdigest()
        for p in sorted(folder.rglob("*"))
        if p.is_file()
    }


REVISION_FILES = [
    "analysis.json",
    "baseline.wav",
    "baseline_evaluation.json",
    "best.json",
    "iter_00",
    "report.md",
    "revision.json",
    "source_evaluation.json",
    "source_plan.json",
]


@pytest.mark.req("REV-1")
def test_revise_run_makes_one_decision(tmp_path: Path) -> None:
    from stemwise.runs import revise_run

    root = _loop_project(tmp_path / "song")
    source = mix_run(root, MockDecider(), max_iterations=2, when=NOW)
    before = _hashes(source.run_dir)
    decider = Recording(MockDecider())
    run = revise_run(root, decider, "  voce più avanti ", when=NOW)
    assert run.run_dir != source.run_dir
    assert sorted(p.name for p in run.run_dir.iterdir()) == REVISION_FILES
    assert len(run.iterations) == 1
    (inp,) = decider.inputs
    start = source.iterations[source.best.iteration].decision.plan
    assert inp.request == "voce più avanti"
    assert inp.previous_plan == start
    assert [h.plan for h in inp.history] == [start]
    assert inp.history[0].evaluation == _read(source.best_iteration.evaluation_file)
    for name in ("analysis.json", "baseline.wav", "baseline_evaluation.json"):
        assert (run.run_dir / name).read_bytes() == (source.run_dir / name).read_bytes(), name
    assert (
        run.run_dir / "source_plan.json"
    ).read_bytes() == source.best_iteration.plan_file.read_bytes()
    assert (run.run_dir / "source_evaluation.json").read_bytes() == (
        source.best_iteration.evaluation_file.read_bytes()
    )
    assert _read(run.run_dir / "revision.json") == {
        "source_run": source.run_dir.name,
        "source_iteration": source.best.iteration,
        "source_plan": source.best.plan,
        "request": "voce più avanti",
    }
    best = _read(run.best_file)
    assert best["iteration"] == 0
    assert best["stop_reason"] == "revision"
    plan = _read(run.iterations[0].plan_file)
    assert plan["iteration"] == 0
    assert plan["stems"]["lead_vox"]["gain_db"] == start.stems["lead_vox"].gain_db + 2.0
    assert _hashes(source.run_dir) == before  # the source run is untouched (PRD §8)


@pytest.mark.req("REV-1")
def test_revise_applies_change_limits(tmp_path: Path) -> None:
    from stemwise.runs import revise_run

    root = _loop_project(tmp_path / "song")
    source = mix_run(root, MockDecider(), max_iterations=1, when=NOW)
    doc = source.iterations[0].decision.plan.model_dump(mode="json")
    start_gain = doc["stems"]["lead_vox"]["gain_db"]
    doc["stems"]["lead_vox"]["gain_db"] = start_gain + 10.0
    run = revise_run(root, Returns(doc), "vocal much louder", when=NOW)
    plan = _read(run.iterations[0].plan_file)
    assert plan["stems"]["lead_vox"]["gain_db"] == start_gain + 4.0  # DEC-5 (D-063)
    request = _read(run.iterations[0].dir / "llm_request.json")
    assert any("lead_vox.gain_db" in w for w in request["warnings"])


@pytest.mark.req("REV-2")
def test_revise_has_no_rollback(tmp_path: Path) -> None:
    from stemwise.runs import revise_run

    root = _loop_project(tmp_path / "song")
    source = mix_run(root, MockDecider(), when=NOW)
    doc = source.best_iteration.decision.plan.model_dump(mode="json")
    for sp in doc["stems"].values():
        sp["lpf_hz"] = 2000.0  # no highs at all: much farther from the pink-noise target
    run = revise_run(root, Returns(doc), "darker", when=NOW)
    evaluation = _read(run.iterations[0].evaluation_file)
    assert evaluation["spectral_distance_db"] > source.best.spectral_distance_db + 1.0
    assert evaluation["is_best"] is True
    assert evaluation["stop_reason"] == "revision"
    assert _read(run.best_file)["iteration"] == 0


@pytest.mark.req("REV-2")
def test_revise_violating_constraints_has_no_best(tmp_path: Path) -> None:
    from stemwise.runs import plan_run, revise_run
    from tests.helpers.plan import neutral_plan
    from tests.helpers.project import StemFixture, default_stems
    from tests.helpers.signals import sine

    stems = [s for s in default_stems() if s.id != "lead_vox"]
    dc = sine(1000, amplitude=0.2) + 0.005  # 0.5% DC: removed only by a high-pass (D-053)
    stems.append(StemFixture("lead_vox", "lead_vocal", dc))
    root = make_project(tmp_path / "song", stems, options={"pre_normalize": False})
    ids = [s.id for s in stems]
    start = neutral_plan(ids)
    start["stems"]["lead_vox"]["hpf_hz"] = 20.0
    plan_file = tmp_path / "start.json"
    plan_file.write_text(json.dumps(start), encoding="utf-8")
    source = plan_run(root, plan_file, when=NOW)
    assert source.best.iteration == 0  # the start is within the constraints
    run = revise_run(root, Returns(neutral_plan(ids)), "no filters", when=NOW)
    evaluation = _read(run.iterations[0].evaluation_file)
    assert evaluation["constraints"]["dc_ok"] is False
    assert _read(run.best_file)["iteration"] is None
    assert run.best.iteration is None
    assert (run.iterations[0].mix_file).is_file()  # kept for listening


# --- master (M6, D-072 → D-075) ------------------------------------------------------------------


def _mastered_project(root: Path) -> Path:
    root = _loop_project(root)
    mix_run(root, MockDecider(), when=NOW)
    return root


@pytest.mark.req("MAS-2", "MAS-3")
def test_master_run_files(tmp_path: Path) -> None:
    from stemwise.master import MasterReport
    from stemwise.runs import master_run

    root = _mastered_project(tmp_path / "song")
    result = master_run(root)
    assert result.master_dir == root / "runs" / "2026-11-09T21-30-00" / "master"
    assert sorted(p.name for p in result.master_dir.iterdir()) == [
        "master.wav",
        "master_report.json",
    ]
    report = MasterReport.model_validate_json(result.report_file.read_text(encoding="utf-8"))
    assert report == result.report
    assert report.files == ["master.wav"]
    assert report.reference.id == "ref1"


@pytest.mark.req("MAS-2")
def test_master_pcm16(tmp_path: Path) -> None:
    """D-075: --pcm16 adds a 16-bit copy of the same signal; the 24-bit file stays."""
    import numpy as np
    import soundfile as sf

    from stemwise.runs import master_run

    root = _mastered_project(tmp_path / "song")
    result = master_run(root, pcm16=True)
    assert result.report.files == ["master.wav", "master_16.wav"]
    hi, sr = sf.read(str(result.master_dir / "master.wav"), dtype="float64")
    lo, sr16 = sf.read(str(result.master_dir / "master_16.wav"), dtype="float64")
    assert sf.info(str(result.master_dir / "master_16.wav")).subtype == "PCM_16"
    assert sf.info(str(result.master_dir / "master.wav")).subtype == "PCM_24"
    assert sr == sr16 == 48_000
    np.testing.assert_allclose(lo, hi, rtol=0, atol=2.0**-15)  # 1 LSB at 16 bit


@pytest.mark.req("MAS-2")
def test_master_is_deterministic(tmp_path: Path) -> None:
    from stemwise.runs import master_run

    root = _mastered_project(tmp_path / "song")
    first, second = master_run(root), master_run(root)
    assert _hashes(first.master_dir)["master.wav"] == _hashes(second.master_dir)["master.wav"]


@pytest.mark.req("MAS-1")
def test_master_never_overwrites(tmp_path: Path) -> None:
    """D-072: master/, then master_02/; nothing already in the run (or in stems/) changes."""
    from stemwise.runs import master_run

    root = _mastered_project(tmp_path / "song")
    (run_dir,) = (root / "runs").iterdir()
    before = {**_hashes(run_dir), **_hashes(root / "stems"), **_hashes(root / "references")}
    first = master_run(root)
    second = master_run(root)
    third = master_run(root)
    assert [first.master_dir.name, second.master_dir.name, third.master_dir.name] == [
        "master",
        "master_02",
        "master_03",
    ]
    after = {**_hashes(run_dir), **_hashes(root / "stems"), **_hashes(root / "references")}
    assert {k: v for k, v in after.items() if not k.startswith("master")} == before


@pytest.mark.req("MAS-1")
def test_master_of_an_external_mix(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    """D-072: --mix writes a new run without best.json, which never becomes the latest run."""
    import shutil
    from datetime import timedelta

    from stemwise.report import latest_run
    from stemwise.runs import master_run

    root = _mastered_project(tmp_path / "song")
    (run_dir,) = (root / "runs").iterdir()
    best = _read(run_dir / "best.json")
    external = tmp_path / "manual_mix.wav"
    shutil.copyfile(run_dir / best["plan"].replace("plan.json", "mix.wav"), external)

    result = master_run(root, mix=external, when=NOW + timedelta(minutes=1))
    assert result.run_dir == root / "runs" / "2026-11-09T21-31-00"
    assert sorted(p.name for p in result.run_dir.iterdir()) == ["master"]
    assert result.report.mix.run is None
    assert result.report.mix.iteration is None
    assert result.report.mix.file == str(external)
    assert latest_run(root) == run_dir

    from stemwise.master import MasterError

    with pytest.raises(MasterError, match="together"):
        master_run(root, mix=external, run=Path(run_dir.name))
