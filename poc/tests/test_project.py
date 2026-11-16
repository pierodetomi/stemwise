"""Loading and validation of project.yaml (ING-1, D-010)."""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

import pytest
import yaml

from stemwise.errors import ProjectError
from stemwise.project import ProjectSpec, load_project
from stemwise.schema_export import CONTRACTS, render_schema
from tests.helpers.project import default_stems, make_project, project_dict

ROOT = Path(__file__).resolve().parent.parent


def _write_yaml(root: Path, doc: Any) -> Path:
    root.mkdir(parents=True, exist_ok=True)
    (root / "project.yaml").write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")
    return root


@pytest.mark.req("ING-1")
def test_load_valid_project(tmp_path: Path) -> None:
    root = make_project(tmp_path / "song")
    project = load_project(root)
    assert project.root == root.resolve()
    assert project.spec.name == "Synthetic song"
    assert project.spec.sample_rate == 48_000
    assert [s.id for s in project.spec.stems] == ["kick", "bass", "gtr", "lead_vox"]
    assert project.stem_path(project.spec.stems[0]) == (root / "stems" / "kick.wav").resolve()
    # Defaults of PRD §6.1.
    assert project.spec.options.pre_normalize is True
    assert project.spec.options.max_iterations == 3
    assert project.spec.options.mix_true_peak_db == -3.0
    assert project.spec.references == []


@pytest.mark.req("ING-1")
def test_sample_rate_auto_accepted(tmp_path: Path) -> None:
    root = make_project(tmp_path / "song", sample_rate="auto")
    assert load_project(root).spec.sample_rate == "auto"


def _mutations() -> list[tuple[str, Any, str]]:
    """(case id, function mutating a valid doc, text the error must contain)."""

    def drop(key):
        return lambda d: d.pop(key)

    def set_(path, value):
        def apply(d):
            target = d
            for k in path[:-1]:
                target = target[k]
            target[path[-1]] = value

        return apply

    def stem_count(n):
        def apply(d):
            base = d["stems"][0]
            d["stems"] = [{**base, "id": f"s{i}"} for i in range(n)]

        return apply

    return [
        ("missing name", drop("name"), "name"),
        ("missing sample_rate", drop("sample_rate"), "sample_rate"),
        ("missing brief", drop("brief"), "brief"),
        ("missing stems", drop("stems"), "stems"),
        ("bad sample_rate", set_(["sample_rate"], "fast"), "sample_rate"),
        ("negative sample_rate", set_(["sample_rate"], -1), "sample_rate"),
        ("unknown role", set_(["stems", 0, "role"], "cowbell"), "stems.0.role"),
        ("bad id", set_(["stems", 0, "id"], "Lead Vox"), "stems.0.id"),
        ("duplicate id", set_(["stems", 1, "id"], "kick"), "kick"),
        ("unknown field", set_(["colour"], "blue"), "colour"),
        ("unknown stem field", set_(["stems", 0, "volume"], 3), "stems.0.volume"),
        ("one stem", stem_count(1), "stems"),
        ("seventeen stems", stem_count(17), "stems"),
        (
            "four references",
            set_(["references"], [{"file": "stems/kick.wav"}] * 4),
            "references",
        ),
        ("true peak too high", set_(["options"], {"mix_true_peak_db": -1.0}), "mix_true_peak_db"),
        ("true peak too low", set_(["options"], {"mix_true_peak_db": -30.0}), "mix_true_peak_db"),
        (
            "true peak -inf",
            set_(["options"], {"mix_true_peak_db": float("-inf")}),
            "mix_true_peak_db",
        ),
        (
            "true peak nan",
            set_(["options"], {"mix_true_peak_db": float("nan")}),
            "mix_true_peak_db",
        ),
        ("id too long", set_(["stems", 0, "id"], "a" * 33), "stems.0.id"),
        ("too many iterations", set_(["options"], {"max_iterations": 4}), "max_iterations"),
        ("zero iterations", set_(["options"], {"max_iterations": 0}), "max_iterations"),
    ]


@pytest.mark.req("ING-1")
@pytest.mark.parametrize(
    ("mutate", "expected"), [m[1:] for m in _mutations()], ids=[m[0] for m in _mutations()]
)
def test_invalid_project_reports_field(tmp_path: Path, mutate, expected: str) -> None:
    root = make_project(tmp_path / "song")
    doc = copy.deepcopy(project_dict(default_stems()))
    mutate(doc)
    _write_yaml(root, doc)
    with pytest.raises(ProjectError, match=r"project\.yaml") as info:
        load_project(root)
    assert expected in str(info.value)


@pytest.mark.req("ING-1")
def test_missing_stem_file_reports_path(tmp_path: Path) -> None:
    root = make_project(tmp_path / "song")
    (root / "stems" / "bass.wav").unlink()
    with pytest.raises(ProjectError) as info:
        load_project(root)
    message = str(info.value)
    assert "bass" in message
    assert str(root.resolve() / "stems" / "bass.wav") in message


@pytest.mark.req("ING-1")
def test_missing_reference_file_reports_path(tmp_path: Path) -> None:
    root = make_project(tmp_path / "song", references=None)
    doc = project_dict(default_stems(), references=[{"file": "references/nope.wav"}])
    _write_yaml(root, doc)
    with pytest.raises(ProjectError, match=r"nope\.wav"):
        load_project(root)


@pytest.mark.req("ING-1")
def test_missing_project_yaml(tmp_path: Path) -> None:
    with pytest.raises(ProjectError, match=r"project\.yaml"):
        load_project(tmp_path)


@pytest.mark.req("ING-1")
@pytest.mark.parametrize("content", ["name: [unclosed", "- just\n- a list\n", ""])
def test_unreadable_yaml(tmp_path: Path, content: str) -> None:
    (tmp_path / "project.yaml").write_text(content, encoding="utf-8")
    with pytest.raises(ProjectError, match=r"project\.yaml"):
        load_project(tmp_path)


@pytest.mark.req("ING-1")
def test_committed_schema_matches_model() -> None:
    """schemas/ is the neutral contract (PRD §7.1): it must not drift from the Pydantic model."""
    assert CONTRACTS["project.schema.json"] is ProjectSpec
    for name, model in CONTRACTS.items():
        committed = (ROOT / "schemas" / name).read_text(encoding="utf-8")
        assert committed == render_schema(model), f"run `make schemas` ({name} is stale)"
        assert "pydantic" not in committed.lower()
        json.loads(committed)
