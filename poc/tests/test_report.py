"""report.md of a run (REP-1, D-056): built from the files of the run only."""

from __future__ import annotations

import copy
import json
import re
from datetime import datetime
from pathlib import Path

import pytest

from stemwise.decision.mock import MockDecider
from stemwise.report import ReportError, latest_run, load_report_data, render_report, write_report
from stemwise.runs import create_run_dir, mix_run
from tests.helpers.openrouter import FakeOpenRouter, completion
from tests.helpers.plan import example_plan
from tests.helpers.project import make_project
from tests.helpers.signals import pink_noise

pytestmark = pytest.mark.req("REP-1")

NOW = datetime(2026, 11, 10, 20, 0, 0)
IDS = ["kick", "bass", "gtr", "lead_vox"]


def _project(root: Path) -> Path:
    reference = pink_noise(duration_s=4.0, channels=2, rms=0.1, seed=9)
    return make_project(root, references=[("ref.wav", reference, 48_000)])


def _llm(fake: FakeOpenRouter):
    from stemwise.decision.config import LlmConfig
    from stemwise.decision.llm import LlmDecider
    from tests.helpers.openrouter import KEY, MODEL

    return LlmDecider(LlmConfig(model=MODEL, api_key=KEY), transport=fake.transport())


def _rows(text: str) -> dict[str, list[str]]:
    """Rows of the iterations table: first cell (without bold) -> other cells."""
    section = text.split("## Iterations", 1)[1].split("## Summaries", 1)[0]
    rows = {}
    for line in section.splitlines():
        if line.startswith("| ") and not line.startswith("| ---"):
            cells = [c.strip() for c in line.strip("|").split("|")]
            label = re.sub(r"\*\*(\S+) \(best\)\*\*", r"\1", cells[0])
            rows[label] = cells[1:]
    rows.pop("", None)  # the header
    return rows


def _read(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_table_has_the_raw_sum_and_every_iteration(tmp_path: Path) -> None:
    run = mix_run(_project(tmp_path / "song"), MockDecider(), when=NOW)
    text = run.report_file.read_text(encoding="utf-8")
    rows = _rows(text)
    assert list(rows) == ["raw sum", *(it.dir.name for it in run.iterations)]
    for label, ev_file in [
        ("raw sum", run.run_dir / "baseline_evaluation.json"),
        *((it.dir.name, it.evaluation_file) for it in run.iterations),
    ]:
        ev = _read(ev_file)
        distance, lufs, true_peak = rows[label][:3]
        assert distance == f"{ev['spectral_distance_db']:.1f}"
        assert lufs == f"{ev['mix']['lufs']:.1f}"
        assert true_peak == f"{ev['mix']['true_peak_db']:.1f}"
    best = run.best.iteration
    assert f"**iter_{best:02d} (best)**" in text
    assert "- Decider: mock" in text
    assert "- Cost of the model calls: no model call" in text


def test_best_plan_rationales_and_collisions(tmp_path: Path) -> None:
    run = mix_run(_project(tmp_path / "song"), MockDecider(), when=NOW)
    text = run.report_file.read_text(encoding="utf-8")
    best = run.best_iteration
    assert f"## Best plan (iter_{best.iteration:02d})" in text
    for sid, sp in best.decision.plan.stems.items():
        assert f"| {sid} |" in text
        assert " ".join(sp.rationale.split()) in text
    assert "## Main collisions" in text
    assert f"Left in the best mix (iter_{best.iteration:02d}):" in text
    first = run.baseline_evaluation.masking_top[0]
    assert f"| {first.a} – {first.b} | {first.band} Hz | {first.score:.2f} |" in text


def test_cost_is_the_sum_of_the_calls(tmp_path: Path) -> None:
    a = example_plan(IDS)
    b = copy.deepcopy(a)
    b["stems"]["gtr"]["gain_db"] += 4.0  # closer to the pink reference: a second iteration
    fake = FakeOpenRouter(["nope", json.dumps(a), json.dumps(b), json.dumps(a)])
    run = mix_run(_project(tmp_path / "song"), _llm(fake), when=NOW)
    text = run.report_file.read_text(encoding="utf-8")
    assert len(fake.chat_requests) == 4
    assert "0.0084 USD, 4 call(s), 4000 prompt + 800 completion tokens" in text  # 4 × 0.0021
    assert "- Decider: llm (vendor/test-model)" in text
    assert _rows(text)["iter_00"][-1] == "0.0042"  # two attempts


def test_partial_cost(tmp_path: Path) -> None:
    usage = {"prompt_tokens": 1000, "completion_tokens": 200, "total_tokens": 1200}
    no_cost = completion("nope", usage=usage)
    fake = FakeOpenRouter([no_cost, json.dumps(example_plan(IDS))])
    root = make_project(tmp_path / "song")  # no reference: one iteration
    run = mix_run(root, _llm(fake), when=NOW)
    text = run.report_file.read_text(encoding="utf-8")
    assert "0.0021 USD (partial: some calls did not report their cost)" in text
    assert "references: none (only the technical constraints are checked)" in text
    assert "stop: no reference: stopped at the first iteration within the constraints" in text


def test_failed_decision_and_warnings(tmp_path: Path) -> None:
    jump = example_plan(IDS)
    jump["stems"]["gtr"]["gain_db"] = 10.0  # clipped to +6 dB (DEC-5): closer to the reference
    fake = FakeOpenRouter([json.dumps(example_plan(IDS)), json.dumps(jump), "x", "x", "x"])
    run = mix_run(_project(tmp_path / "song"), _llm(fake), max_iterations=3, when=NOW)
    text = run.report_file.read_text(encoding="utf-8")
    assert run.failed_iteration == 2
    rows = _rows(text)
    assert rows[f"iter_{run.failed_iteration:02d}"][0] == "decision failed"
    assert "stop: the decision of the next iteration failed" in text
    assert "## Warnings" in text
    assert "iter_01: stems.gtr.gain_db: +10.0 dB is more than 4 dB away" in text


def test_regeneration_is_identical(tmp_path: Path) -> None:
    run = mix_run(_project(tmp_path / "song"), MockDecider(), when=NOW)
    before = run.report_file.read_bytes()
    run.report_file.unlink()
    assert write_report(run.run_dir).read_bytes() == before
    assert render_report(load_report_data(run.run_dir)).encode() == before


def test_runs_without_evaluations_are_refused(tmp_path: Path) -> None:
    old = create_run_dir(tmp_path, NOW)  # e.g. a run of M3, or `analyze`
    with pytest.raises(ReportError, match=r"no best\.json"):
        load_report_data(old)
    with pytest.raises(ReportError, match="run `stemwise mix` first"):
        latest_run(tmp_path)


def test_latest_run_has_a_best_file(tmp_path: Path) -> None:
    root = _project(tmp_path / "song")
    first = mix_run(root, MockDecider(), max_iterations=1, when=NOW)
    second = mix_run(root, MockDecider(), max_iterations=1, when=NOW)  # same second: "-2"
    later = create_run_dir(root, datetime(2026, 11, 10, 21, 0, 0))  # no best.json
    assert second.run_dir.name.endswith("-2")
    assert latest_run(root) == second.run_dir
    assert later.is_dir()
    assert first.run_dir != second.run_dir


def test_malformed_files_give_a_clear_error(tmp_path: Path) -> None:
    run = mix_run(_project(tmp_path / "song"), MockDecider(), max_iterations=1, when=NOW)
    path = run.iterations[0].evaluation_file
    doc = _read(path)
    doc["spectral_distance_db"] = -1
    path.write_text(json.dumps(doc), encoding="utf-8")
    with pytest.raises(ReportError, match=r"not a valid evaluation\.json"):
        load_report_data(run.run_dir)


# --- revision report (REV-3, D-064, D-066) --------------------------------------------------------


class _Returns:
    name = "scripted"

    def __init__(self, doc: dict) -> None:
        self.doc = doc

    def decide(self, decision_input):
        from stemwise.decision.base import Decision
        from stemwise.decision.schema import MixPlan

        return Decision(plan=MixPlan.model_validate(self.doc), request_log={"decider": self.name})


def _section(text: str, title: str) -> str:
    return text.split(f"## {title}", 1)[1].split("\n## ", 1)[0]


def _table(section: str) -> list[list[str]]:
    rows = []
    for line in section.splitlines():
        if line.startswith("| ") and not line.startswith("| ---"):
            rows.append([c.strip() for c in line.strip("|").split("|")])
    return rows[1:]  # without the header


def _revised(tmp_path: Path):
    from stemwise.runs import revise_run

    root = _project(tmp_path / "song")
    source = mix_run(root, MockDecider(), max_iterations=1, when=NOW)
    doc = source.iterations[0].decision.plan.model_dump(mode="json")
    start = copy.deepcopy(doc)
    doc["stems"]["lead_vox"]["gain_db"] += 10.0  # clipped to +4 (DEC-5)
    doc["stems"]["lead_vox"]["rationale"] = "Più avanti, come chiesto."
    doc["stems"]["gtr"]["reverb_send_db"] = None
    doc["stems"]["gtr"]["rationale"] = "Niente riverbero sulla chitarra."
    doc["summary"] = "Voce più avanti, chitarra asciutta."
    run = revise_run(root, _Returns(doc), "voce più avanti, chitarra asciutta", when=NOW)
    return source, run, start


@pytest.mark.req("REV-3")
def test_revision_report(tmp_path: Path) -> None:
    from stemwise.decision.schema import MixPlan
    from stemwise.revise import plan_diff, unchanged_stems

    source, run, start_doc = _revised(tmp_path)
    text = run.report_file.read_text(encoding="utf-8")
    assert text.startswith(f"# Revision report — run {run.run_dir.name}\n")
    assert '- Request: "voce più avanti, chitarra asciutta"' in text
    assert f"- Starts from: run {source.run_dir.name}, iter_00 (its best plan)" in text
    assert "- Decider: scripted" in text
    # Comparison table: values from the files.
    rows = {r[0].strip("*"): r[1:] for r in _table(_section(text, "Comparison"))}
    assert list(rows) == ["raw sum", "starting plan", "revised plan (iter_00)"]
    for label, ev in [
        ("raw sum", _read(run.run_dir / "baseline_evaluation.json")),
        ("starting plan", _read(run.run_dir / "source_evaluation.json")),
        ("revised plan (iter_00)", _read(run.iterations[0].evaluation_file)),
    ]:
        assert rows[label][:3] == [
            f"{ev['spectral_distance_db']:.1f}",
            f"{ev['mix']['lufs']:.1f}",
            f"{ev['mix']['true_peak_db']:.1f}",
        ]
    # Diff: one row per change of plan_diff, in its order.
    start = MixPlan.model_validate(start_doc)
    revised = MixPlan.model_validate(_read(run.iterations[0].plan_file))
    changes = plan_diff(start, revised)
    diff_rows = _table(_section(text, "Changes from the starting plan"))
    assert [(r[0], r[1]) for r in diff_rows] == [(c.target, c.parameter) for c in changes]
    gain = start.stems["lead_vox"].gain_db
    assert ["lead_vox", "gain_db", f"{gain:+.1f} dB", f"{gain + 4:+.1f} dB"] in diff_rows
    assert ["gtr", "reverb_send_db", "-24.0 dB", "off"] in diff_rows
    unchanged = ", ".join(unchanged_stems(start, revised))
    assert f"Unchanged stems: {unchanged}." in text
    rationales = _section(text, "Rationales of the changed stems")
    assert "| lead_vox | lead_vocal | Più avanti, come chiesto. |" in rationales
    assert "| gtr | electric_guitar | Niente riverbero sulla chitarra. |" in rationales
    assert "kick" not in rationales
    assert "- revised plan: Voce più avanti, chitarra asciutta." in text
    assert "lead_vox.gain_db: +" in _section(text, "Warnings")  # DEC-5 clip
    assert "Left in the revised mix (iter_00):" in text
    # Regenerated identically from the files of the run.
    before = run.report_file.read_bytes()
    run.report_file.unlink()
    assert write_report(run.run_dir).read_bytes() == before


@pytest.mark.req("REV-3")
def test_revision_report_without_changes(tmp_path: Path) -> None:
    from stemwise.runs import revise_run

    root = _project(tmp_path / "song")
    mix_run(root, MockDecider(), max_iterations=1, when=NOW)
    run = revise_run(root, MockDecider(), "make it sound like the eighties", when=NOW)
    text = run.report_file.read_text(encoding="utf-8")
    assert "No parameter changed." in text
    assert "Unchanged stems: kick, bass, gtr, lead_vox." in text
