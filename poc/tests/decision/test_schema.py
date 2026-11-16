"""Mix plan contract (PRD §6.3, D-027, D-033) and the per-project schema for the model (DEC-2)."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

from stemwise.decision.schema import (
    UNSUPPORTED_STRICT_KEYWORDS,
    Compressor,
    EqBand,
    MixPlan,
    Reverb,
    StemPlan,
    check_plan_fits,
    dumps_plan,
    load_plan,
    parse_plan,
    project_plan_schema,
)
from stemwise.errors import PlanError
from stemwise.schema_export import CONTRACTS
from tests.helpers.plan import example_plan, neutral_plan, write_plan


def _set(doc: dict[str, Any], path: str, value: Any) -> dict[str, Any]:
    node: Any = doc
    *parents, last = path.split(".")
    for key in parents:
        node = node[int(key)] if isinstance(node, list) else node[key]
    if isinstance(node, list):
        node[int(last)] = value
    else:
        node[last] = value
    return doc


def test_example_plan_of_prd_is_valid() -> None:
    plan = MixPlan.model_validate(example_plan())
    vox = plan.stems["lead_vox"]
    assert vox.hpf_hz == 90
    assert [b.type for b in vox.eq_bands] == ["peak", "high_shelf"]
    assert vox.compressor is not None
    assert vox.compressor.makeup_db == 3.0
    assert plan.reverb is not None
    assert plan.reverb.return_gain_db == -6.0


def test_null_switches_blocks_off() -> None:
    plan = MixPlan.model_validate(neutral_plan(["a", "b"]))
    assert plan.reverb is None
    for sp in plan.stems.values():
        assert sp.hpf_hz is None
        assert sp.compressor is None
        assert sp.reverb_send_db is None
        assert sp.eq_bands == []
    doc = neutral_plan(["a"])
    doc["stems"]["a"]["eq"] = None
    assert MixPlan.model_validate(doc).stems["a"].eq_bands == []


# (path, value just inside, value just outside) for every limit of PRD §6.3.
LIMITS = [
    ("stems.lead_vox.gain_db", -24.0, -24.01),
    ("stems.lead_vox.gain_db", 12.0, 12.01),
    ("stems.lead_vox.pan", -1.0, -1.01),
    ("stems.lead_vox.pan", 1.0, 1.01),
    ("stems.lead_vox.hpf_hz", 20.0, 19.9),
    ("stems.lead_vox.hpf_hz", 400.0, 400.1),
    ("stems.lead_vox.lpf_hz", 2_000.0, 1_999.0),
    ("stems.lead_vox.lpf_hz", 20_000.0, 20_001.0),
    ("stems.lead_vox.eq.0.freq_hz", 20.0, 19.9),
    ("stems.lead_vox.eq.0.freq_hz", 20_000.0, 20_001.0),
    ("stems.lead_vox.eq.0.gain_db", -9.0, -9.1),
    ("stems.lead_vox.eq.0.gain_db", 9.0, 9.1),
    ("stems.lead_vox.eq.0.q", 0.3, 0.29),
    ("stems.lead_vox.eq.0.q", 8.0, 8.1),
    ("stems.lead_vox.compressor.threshold_db", -40.0, -40.1),
    ("stems.lead_vox.compressor.threshold_db", 0.0, 0.1),
    ("stems.lead_vox.compressor.ratio", 1.0, 0.99),
    ("stems.lead_vox.compressor.ratio", 8.0, 8.1),
    ("stems.lead_vox.compressor.attack_ms", 0.1, 0.09),
    ("stems.lead_vox.compressor.attack_ms", 100.0, 100.1),
    ("stems.lead_vox.compressor.release_ms", 20.0, 19.9),
    ("stems.lead_vox.compressor.release_ms", 1_000.0, 1_000.1),
    ("stems.lead_vox.compressor.makeup_db", 0.0, -0.1),
    ("stems.lead_vox.compressor.makeup_db", 12.0, 12.1),
    ("stems.lead_vox.reverb_send_db", -60.0, -60.1),
    ("stems.lead_vox.reverb_send_db", -6.0, -5.9),
    ("reverb.room_size", 0.0, -0.01),
    ("reverb.room_size", 1.0, 1.01),
    ("reverb.damping", 0.0, -0.01),
    ("reverb.damping", 1.0, 1.01),
    ("reverb.return_hpf_hz", 20.0, 19.9),
    ("reverb.return_hpf_hz", 800.0, 800.1),
    ("reverb.return_gain_db", -24.0, -24.1),
    ("reverb.return_gain_db", 0.0, 0.1),
]


@pytest.mark.parametrize(("path", "inside", "outside"), LIMITS)
def test_parameter_limits(path: str, inside: float, outside: float) -> None:
    MixPlan.model_validate(_set(example_plan(), path, inside))
    with pytest.raises(PlanError, match=path.replace(".", r"\.")):
        parse_plan(json.dumps(_set(example_plan(), path, outside)))


def test_eq_band_count_and_type() -> None:
    doc = example_plan()
    band = doc["stems"]["lead_vox"]["eq"][0]
    doc["stems"]["lead_vox"]["eq"] = [band] * 4
    MixPlan.model_validate(doc)
    doc["stems"]["lead_vox"]["eq"] = [band] * 5
    with pytest.raises(PlanError, match="eq"):
        parse_plan(json.dumps(doc))
    with pytest.raises(PlanError, match="type"):
        parse_plan(json.dumps(_set(example_plan(), "stems.lead_vox.eq.0.type", "notch")))


@pytest.mark.parametrize(
    "mutate",
    [
        lambda d: d.pop("summary"),
        lambda d: d["stems"]["lead_vox"].pop("rationale"),
        lambda d: d["stems"]["lead_vox"].pop("compressor"),  # complete plan: no defaults
        lambda d: d["stems"]["lead_vox"].update(saturation=1.0),  # unknown field
        lambda d: d.update(version=2),
        lambda d: d["stems"]["lead_vox"].update(rationale=""),
        lambda d: d["stems"].update({"Bad-Id": d["stems"]["lead_vox"]}),
    ],
    ids=["no-summary", "no-rationale", "missing-block", "unknown-field", "version", "empty", "id"],
)
def test_invalid_plans_rejected(mutate: Any) -> None:
    doc = example_plan()
    mutate(doc)
    with pytest.raises(PlanError, match="invalid mix plan"):
        parse_plan(json.dumps(doc))


def test_non_json_and_non_object() -> None:
    with pytest.raises(PlanError, match="not valid JSON"):
        parse_plan("gain: 3")
    with pytest.raises(PlanError, match="JSON object"):
        parse_plan("[1, 2]")
    with pytest.raises(PlanError, match="not valid JSON"):
        parse_plan('{"version": 1, "x": NaN}'.replace("NaN", "nan"))


def test_nan_and_inf_rejected() -> None:
    with pytest.raises(PlanError):
        parse_plan(json.dumps(_set(example_plan(), "stems.lead_vox.gain_db", float("nan"))))


def test_load_and_dump_roundtrip(tmp_path: Path) -> None:
    path = write_plan(tmp_path / "plan.json", example_plan(["kick", "lead_vox"]))
    plan = load_plan(path)
    assert parse_plan(dumps_plan(plan)) == plan
    with pytest.raises(PlanError, match="not found"):
        load_plan(tmp_path / "missing.json")


def test_plan_must_list_exactly_the_project_stems() -> None:
    plan = MixPlan.model_validate(neutral_plan(["kick", "extra"]))
    with pytest.raises(PlanError) as err:
        check_plan_fits(plan, ["kick", "bass"], 48_000)
    assert "missing from the plan: bass" in str(err.value)
    assert "not in the project: extra" in str(err.value)
    check_plan_fits(
        MixPlan.model_validate(neutral_plan(["kick", "bass"])), ["kick", "bass"], 48_000
    )


def test_frequencies_must_be_below_nyquist() -> None:
    plan = MixPlan.model_validate(_set(example_plan(), "stems.lead_vox.lpf_hz", 16_000))
    check_plan_fits(plan, ["lead_vox"], 44_100)
    with pytest.raises(PlanError, match=r"stems\.lead_vox\.lpf_hz = 16000 Hz"):
        check_plan_fits(plan, ["lead_vox"], 32_000)


def test_committed_schema_registered() -> None:
    # The drift check itself is tests/test_project.py::test_committed_schema_matches_model.
    assert CONTRACTS["mix_plan.schema.json"] is MixPlan


@pytest.mark.parametrize(
    ("path", "value"),
    [
        ("stems.lead_vox.gain_db", "2"),
        ("stems.lead_vox.pan", True),
        ("iteration", "1"),
        ("stems.lead_vox.compressor.ratio", "3"),
    ],
)
def test_no_type_coercion(path: str, value: object) -> None:
    """Strict like the exported JSON Schema: a string or a boolean is not a number."""
    with pytest.raises(PlanError, match=path.replace(".", r"\.")):
        parse_plan(json.dumps(_set(example_plan(), path, value)))


def test_integers_are_valid_numbers() -> None:
    plan = parse_plan(json.dumps(_set(example_plan(), "stems.lead_vox.gain_db", 3)))
    assert plan.stems["lead_vox"].gain_db == 3.0


# --- schema sent to the model (DEC-2, D-039) ---------------------------------------------------


def _walk(node: object):
    if isinstance(node, dict):
        yield node
        for key, value in node.items():
            if key in ("properties", "$defs"):
                for sub in value.values():
                    yield from _walk(sub)
            else:
                yield from _walk(value)
    elif isinstance(node, list):
        for value in node:
            yield from _walk(value)


@pytest.mark.req("DEC-2")
def test_project_schema_lists_exactly_the_project_stems() -> None:
    ids = ["kick", "default", "title", "lead_vox"]  # ids that look like JSON Schema keywords
    schema = project_plan_schema(ids)
    stems = schema["properties"]["stems"]
    assert stems["type"] == "object"
    assert list(stems["properties"]) == ids
    assert stems["required"] == ids
    assert stems["additionalProperties"] is False
    assert all(v == {"$ref": "#/$defs/StemPlan"} for v in stems["properties"].values())
    json.dumps(schema)  # serializable as is


@pytest.mark.req("DEC-2")
def test_project_schema_matches_model() -> None:
    """Same fields, same requirements and same numeric limits as `MixPlan`, strict subset only."""
    schema = project_plan_schema(["a", "b"])
    defs = schema["$defs"]
    models = {"MixPlan": (MixPlan, schema)} | {
        name: (model, defs[name])
        for name, model in [
            ("StemPlan", StemPlan),
            ("EqBand", EqBand),
            ("Compressor", Compressor),
            ("Reverb", Reverb),
        ]
    }
    for name, (model, node) in models.items():
        assert set(node["properties"]) == set(model.model_fields), name
        assert set(node["required"]) == set(model.model_fields), name  # complete plan, no diff
        assert node["additionalProperties"] is False, name
    for node in _walk(schema):
        assert not UNSUPPORTED_STRICT_KEYWORDS & set(node), node
    full = MixPlan.model_json_schema()["$defs"]
    for name in ("StemPlan", "EqBand", "Compressor", "Reverb"):
        for field, prop in full[name]["properties"].items():
            sent = defs[name]["properties"][field]
            for key in ("minimum", "maximum", "enum", "const", "type"):
                assert sent.get(key) == prop.get(key), (name, field, key)
    assert defs["StemPlan"]["properties"]["gain_db"]["minimum"] == -24.0
    assert defs["StemPlan"]["properties"]["gain_db"]["maximum"] == 12.0
    assert schema["properties"]["version"]["const"] == 1


def test_generic_schema_unchanged_by_project_schema() -> None:
    before = MixPlan.model_json_schema()
    project_plan_schema(["a", "b"])
    assert MixPlan.model_json_schema() == before
