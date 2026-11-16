"""Revision in words (M5): contract, plan diff, source run and consistency (REV-1 → REV-3)."""

from __future__ import annotations

import json
import shutil
from datetime import datetime
from pathlib import Path

import pytest
import yaml
from pydantic import ValidationError

from stemwise.analysis.analyze import project_target
from stemwise.audio_io import load_stems
from stemwise.decision.mock import MockDecider
from stemwise.decision.schema import MixPlan
from stemwise.evaluate import Best, Evaluation
from stemwise.project import load_project
from stemwise.report import ReportError, find_run
from stemwise.revise import (
    MAX_REQUEST_CHARS,
    REVERB_TARGET,
    Revision,
    RevisionError,
    check_consistency,
    dumps_revision,
    load_source,
    plan_diff,
    unchanged_stems,
)
from stemwise.runs import mix_run
from stemwise.schema_export import CONTRACTS
from tests.helpers.plan import example_plan
from tests.helpers.project import make_project, write_audio
from tests.helpers.signals import pink_noise, sine

REVISION = {
    "source_run": "2026-11-10T21-30-00",
    "source_iteration": 1,
    "source_plan": "iter_01/plan.json",
    "request": "voce più avanti",
}


# --- contract (D-059) ---------------------------------------------------------------------------


@pytest.mark.req("REV-1")
def test_revision_contract_round_trip() -> None:
    revision = Revision.model_validate(REVISION)
    assert Revision.model_validate_json(dumps_revision(revision)) == revision
    assert dumps_revision(revision).endswith("\n")
    assert "voce più avanti" in dumps_revision(revision)  # not escaped: readable by the author


@pytest.mark.req("REV-1")
@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("request", ""),
        ("request", "   "),
        ("request", "x" * (MAX_REQUEST_CHARS + 1)),
        ("source_iteration", -1),
        ("source_run", ""),
        ("extra", 1),
    ],
)
def test_revision_contract_rejects(field: str, value: object) -> None:
    with pytest.raises(ValidationError):
        Revision.model_validate({**REVISION, field: value})


@pytest.mark.req("REV-1")
def test_request_limit_counts_characters() -> None:
    # 1,000 characters is the limit (D-066), measured on the stripped text.
    ok = {**REVISION, "request": "à" * MAX_REQUEST_CHARS}
    assert Revision.model_validate(ok).request == "à" * MAX_REQUEST_CHARS
    assert MAX_REQUEST_CHARS == 1000


@pytest.mark.req("REV-2")
def test_stop_reason_revision() -> None:
    best = Best(
        iteration=0, plan="iter_00/plan.json", spectral_distance_db=2.0, stop_reason="revision"
    )
    assert best.stop_reason == "revision"
    schema = Evaluation.model_json_schema()
    assert "revision" in str(schema)


@pytest.mark.req("REV-1")
def test_schema_refuses_blank_requests() -> None:
    import re

    request = Revision.model_json_schema()["properties"]["request"]
    assert re.search(request["pattern"], "   ") is None
    assert re.search(request["pattern"], " x ") is not None


@pytest.mark.req("REV-1")
def test_revision_contract_registered() -> None:
    # The drift check itself is tests/test_project.py::test_committed_schema_matches_model.
    assert CONTRACTS["revision.schema.json"] is Revision


# --- plan diff (REV-3, D-064) -------------------------------------------------------------------


def _plan(edit=None, ids=("lead_vox", "gtr")) -> MixPlan:
    doc = example_plan(list(ids))
    if edit is not None:
        edit(doc)
    return MixPlan.model_validate(doc)


def _set(path: str, value: object):
    """An edit of the plan document at a dotted path (`stems.lead_vox.eq.0.gain_db`)."""

    def edit(doc: dict) -> None:
        *head, last = path.split(".")
        node = doc
        for key in head:
            node = node[int(key)] if isinstance(node, list) else node[key]
        if isinstance(node, list):
            if value is _DELETE:
                del node[int(last)]
            else:
                node[int(last)] = value
        elif value is _DELETE:
            del node[last]
        else:
            node[last] = value

    return edit


_DELETE = object()
_BAND = {"type": "peak", "freq_hz": 3000.0, "gain_db": -2.0, "q": 1.0}


@pytest.mark.req("REV-3")
def test_equal_plans_have_no_diff() -> None:
    before = _plan()
    # Same values written differently (90 vs 90.0) and another rationale/summary: no change.
    after = _plan(
        lambda d: (
            d["stems"]["lead_vox"].update(hpf_hz=90.0, rationale="Other words."),
            d.update(summary="Other summary.", iteration=0),
        )
    )
    assert plan_diff(before, after) == []
    assert unchanged_stems(before, after) == ["lead_vox", "gtr"]


@pytest.mark.req("REV-3")
@pytest.mark.parametrize(
    ("edit", "expected"),
    [
        (_set("stems.lead_vox.gain_db", 4.0), [("lead_vox", "gain_db", 2.0, 4.0)]),
        (_set("stems.gtr.pan", -0.5), [("gtr", "pan", 0.0, -0.5)]),
        (_set("stems.lead_vox.hpf_hz", None), [("lead_vox", "hpf_hz", 90.0, None)]),
        (_set("stems.lead_vox.lpf_hz", 12000), [("lead_vox", "lpf_hz", None, 12000.0)]),
        (
            _set("stems.lead_vox.reverb_send_db", -20.0),
            [("lead_vox", "reverb_send_db", -14.0, -20.0)],
        ),
        (
            _set("stems.lead_vox.eq.0.gain_db", -4.0),
            [("lead_vox", "eq[1].gain_db", -2.5, -4.0)],
        ),
        (
            _set("stems.lead_vox.eq.1.freq_hz", 8000),
            [("lead_vox", "eq[2].freq_hz", 9000.0, 8000.0)],
        ),
        (_set("stems.lead_vox.eq.0.q", 2.0), [("lead_vox", "eq[1].q", 1.4, 2.0)]),
        (
            _set("stems.lead_vox.eq.0", {**_BAND, "type": "low_shelf", "freq_hz": 300}),
            [("lead_vox", "eq[1]", "peak 300 Hz -2.5 dB Q 1.4", "low_shelf 300 Hz -2.0 dB Q 1")],
        ),
        (
            lambda d: d["stems"]["lead_vox"]["eq"].append(dict(_BAND)),
            [("lead_vox", "eq[3]", None, "peak 3000 Hz -2.0 dB Q 1")],
        ),
        (
            _set("stems.lead_vox.eq.1", _DELETE),
            [("lead_vox", "eq[2]", "high_shelf 9000 Hz +1.5 dB Q 0.7", None)],
        ),
        (
            _set("stems.lead_vox.compressor.ratio", 4.0),
            [("lead_vox", "compressor.ratio", 3.0, 4.0)],
        ),
        (
            _set("stems.lead_vox.compressor", None),
            [("lead_vox", "compressor", "-18 dB, 3:1, 10/120 ms, makeup 3 dB", None)],
        ),
        (_set("reverb.room_size", 0.5), [(REVERB_TARGET, "room_size", 0.35, 0.5)]),
        (
            _set("reverb", None),
            [
                (
                    REVERB_TARGET,
                    "reverb",
                    "room 0.35, damping 0.5, return high-pass 250 Hz, -6 dB",
                    None,
                )
            ],
        ),
    ],
)
def test_plan_diff(edit, expected) -> None:
    before = _plan()
    after = _plan(edit)
    got = [(c.target, c.parameter, c.before, c.after) for c in plan_diff(before, after)]
    assert got == expected
    changed = {c.target for c in plan_diff(before, after)}
    assert unchanged_stems(before, after) == [s for s in ("lead_vox", "gtr") if s not in changed]


@pytest.mark.req("REV-3")
def test_eq_null_and_empty_are_the_same() -> None:
    before = _plan(_set("stems.gtr.eq", None))
    after = _plan(_set("stems.gtr.eq", []))
    assert plan_diff(before, after) == []


@pytest.mark.req("REV-3")
def test_diff_order_follows_the_plan() -> None:
    def edit(d: dict) -> None:
        d["stems"]["gtr"]["gain_db"] = -1.0
        d["stems"]["lead_vox"]["reverb_send_db"] = -18.0
        d["stems"]["lead_vox"]["gain_db"] = 3.0
        d["reverb"]["damping"] = 0.7

    got = [(c.target, c.parameter) for c in plan_diff(_plan(), _plan(edit))]
    assert got == [
        ("lead_vox", "gain_db"),
        ("lead_vox", "reverb_send_db"),
        ("gtr", "gain_db"),
        (REVERB_TARGET, "damping"),
    ]


def test_diff_needs_the_same_stems() -> None:
    with pytest.raises(ValueError, match="same stems"):
        plan_diff(_plan(), _plan(ids=("lead_vox",)))


@pytest.mark.req("REV-3")
def test_a_stem_called_reverb_is_not_the_shared_reverb() -> None:
    before = _plan(ids=("reverb", "gtr"))
    after = _plan(_set("reverb.damping", 0.9), ids=("reverb", "gtr"))
    assert [c.target for c in plan_diff(before, after)] == [REVERB_TARGET]
    assert unchanged_stems(before, after) == ["reverb", "gtr"]


# --- source run and consistency (REV-1, D-060, D-061) --------------------------------------------

NOW = datetime(2026, 11, 10, 20, 0, 0)


@pytest.fixture(scope="module")
def mixed(tmp_path_factory: pytest.TempPathFactory) -> Path:
    """A synthetic project with a reference and one mock `mix` run of 2 iterations."""
    root = tmp_path_factory.mktemp("mixed") / "song"
    reference = pink_noise(duration_s=4.0, channels=2, rms=0.1, seed=9)
    make_project(root, references=[("ref.wav", reference, 48_000)])
    mix_run(root, MockDecider(), max_iterations=2, when=NOW)
    return root


@pytest.fixture
def song(mixed: Path, tmp_path: Path) -> Path:
    """A private copy of `mixed`, free to modify."""
    return Path(shutil.copytree(mixed, tmp_path / "song"))


def _best(run_dir: Path) -> dict:
    return json.loads((run_dir / "best.json").read_text(encoding="utf-8"))


@pytest.mark.req("REV-1")
def test_source_is_the_best_plan(song: Path) -> None:
    run_dir = find_run(song)
    best = _best(run_dir)
    source = load_source(song, run_dir)
    assert source.iteration == best["iteration"]
    assert source.plan_path == best["plan"]
    assert source.plan == MixPlan.model_validate_json((run_dir / best["plan"]).read_text())
    iter_dir = (run_dir / best["plan"]).parent
    assert source.evaluation_text == (iter_dir / "evaluation.json").read_text(encoding="utf-8")
    assert source.analysis_text == (run_dir / "analysis.json").read_text(encoding="utf-8")
    assert source.label == run_dir.name  # a run of the project: its name
    # --run by name and by path give the same source.
    assert load_source(song, find_run(song, Path(run_dir.name))).plan == source.plan
    outside = Path(shutil.copytree(run_dir, song.parent / "elsewhere"))
    assert load_source(song, find_run(song, outside)).label == str(outside)


@pytest.mark.req("REV-1")
def test_source_without_a_valid_plan(song: Path) -> None:
    run_dir = find_run(song)
    best = {**_best(run_dir), "iteration": None, "plan": None, "spectral_distance_db": None}
    (run_dir / "best.json").write_text(json.dumps(best), encoding="utf-8")
    with pytest.raises(RevisionError, match=r"no plan within the technical constraints.*--run"):
        load_source(song, run_dir)
    (run_dir / "best.json").unlink()
    with pytest.raises(ReportError, match=r"no best\.json"):
        load_source(song, run_dir)
    with pytest.raises(ReportError, match="run `stemwise mix` first"):
        find_run(song)
    with pytest.raises(ReportError, match="run folder not found"):
        find_run(song, Path("nope"))


def _check(root: Path) -> None:
    project = load_project(root)
    stems = load_stems(project)
    roles = {s.id: s.role for s in project.spec.stems}
    source = load_source(root, find_run(root))
    check_consistency(source, stems, roles, project_target(project, stems.sample_rate))


def _edit_yaml(root: Path, edit) -> None:
    path = root / "project.yaml"
    doc = yaml.safe_load(path.read_text(encoding="utf-8"))
    edit(doc)
    path.write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")


def _longer(root: Path) -> None:
    write_audio(root / "stems" / "kick.wav", sine(60, amplitude=0.4, duration_s=2.0))


def _louder(root: Path) -> None:
    write_audio(root / "stems" / "lead_vox.wav", sine(1000, amplitude=0.4))


def _stereo(root: Path) -> None:
    write_audio(root / "stems" / "lead_vox.wav", sine(1000, amplitude=0.2, channels=2))


def _role(root: Path) -> None:
    _edit_yaml(root, lambda d: d["stems"][1].update(role="synth"))


def _renamed(root: Path) -> None:
    shutil.copy(root / "stems" / "bass.wav", root / "stems" / "sub.wav")
    _edit_yaml(root, lambda d: d["stems"][1].update(id="sub", file="stems/sub.wav"))


def _reference(root: Path) -> None:
    write_audio(
        root / "references" / "ref.wav",
        pink_noise(duration_s=4.0, channels=2, rms=0.1, seed=3)[:, ::-1] * [1.0, 0.2],
    )


def _no_reference(root: Path) -> None:
    _edit_yaml(root, lambda d: d.pop("references"))


@pytest.mark.req("REV-1")
def test_consistency_check_passes_on_the_same_project(song: Path) -> None:
    _check(song)  # no error
    # The brief is read again at every revision: changing it is not an inconsistency.
    _edit_yaml(song, lambda d: d["brief"].update(notes="More air on the vocal."))
    _check(song)


@pytest.mark.req("REV-1")
@pytest.mark.parametrize(
    ("change", "message"),
    [
        (_longer, "duration"),
        (_louder, r"lead_vox: peak"),
        (_stereo, r"lead_vox: channels"),
        (_role, r"bass: role"),
        (_renamed, "stems"),
        (_reference, "target"),
        (_no_reference, "references"),
    ],
)
def test_consistency_check(song: Path, change, message: str) -> None:
    run_name = find_run(song).name
    change(song)
    with pytest.raises(RevisionError, match=message) as info:
        _check(song)
    assert run_name in str(info.value)
    assert "stemwise mix" in str(info.value)
