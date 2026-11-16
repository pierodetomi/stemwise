"""Parsing of the model's answer (DEC-2, DEC-3, DEC-4, D-039) and change limits (DEC-5, D-042)."""

from __future__ import annotations

import copy
import json
from typing import Any

import pytest

from stemwise.decision.base import Decision
from stemwise.decision.schema import MixPlan, check_plan_fits
from stemwise.decision.validate import (
    MAX_EQ_CHANGE_DB,
    MAX_GAIN_CHANGE_DB,
    clamp_changes,
    finalize,
    parse_response,
)
from stemwise.errors import PlanError
from tests.helpers.decision import make_input
from tests.helpers.plan import example_plan, neutral_plan

IDS = ["kick", "bass", "gtr", "lead_vox"]
FS = 48_000


def _text(doc: dict[str, Any]) -> str:
    return json.dumps(doc)


def _parse(content: str | None, finish_reason: str | None = "stop") -> MixPlan:
    return parse_response(content, finish_reason, IDS, FS, iteration=1)


def _plan(doc: dict[str, Any]) -> MixPlan:
    return MixPlan.model_validate(doc)


# --- parsing --------------------------------------------------------------------------------


@pytest.mark.req("DEC-2")
def test_valid_answer_is_parsed_and_iteration_imposed() -> None:
    doc = example_plan(IDS)
    doc["iteration"] = 7
    plan = parse_response(_text(doc), "stop", IDS, FS, iteration=2)
    assert plan.iteration == 2  # the engine decides the iteration (D-039)
    assert set(plan.stems) == set(IDS)


@pytest.mark.req("DEC-2")
def test_partial_plan_is_invalid() -> None:
    """DEC-2: always a complete plan, never a diff: a missing stem is an error."""
    doc = example_plan(IDS)
    del doc["stems"]["gtr"]
    with pytest.raises(PlanError, match="stems missing from the plan: gtr"):
        _parse(_text(doc))
    doc = example_plan(IDS)
    del doc["stems"]["lead_vox"]["compressor"]
    with pytest.raises(PlanError, match=r"stems\.lead_vox\.compressor"):
        _parse(_text(doc))


@pytest.mark.req("DEC-3")
@pytest.mark.parametrize(
    ("path", "value"),
    [("stems.gtr.rationale", ""), ("stems.gtr.rationale", "   "), ("summary", " \n")],
)
def test_rationale_and_summary_required(path: str, value: str) -> None:
    doc = example_plan(IDS)
    node: Any = doc
    *parents, last = path.split(".")
    for key in parents:
        node = node[key]
    node[last] = value
    with pytest.raises(PlanError, match=path.replace(".", r"\.")):
        _parse(_text(doc))


@pytest.mark.req("DEC-3")
def test_missing_rationale_is_invalid() -> None:
    doc = example_plan(IDS)
    del doc["stems"]["bass"]["rationale"]
    del doc["summary"]
    with pytest.raises(PlanError) as info:
        _parse(_text(doc))
    assert "stems.bass.rationale" in str(info.value)
    assert "summary" in str(info.value)


@pytest.mark.req("DEC-4")
@pytest.mark.parametrize(
    ("content", "finish_reason", "message"),
    [
        (None, "stop", "empty"),
        ("", "stop", "empty"),
        ("   ", "stop", "empty"),
        ("Sure! Here is the plan you asked for.", "stop", "not valid JSON"),
        ("Here it is: " + _text(example_plan(IDS)), "stop", "not valid JSON"),
        (_text(example_plan(IDS))[:-40], "length", "truncated"),
        (_text(example_plan(IDS)), "length", "truncated"),
        ("[1, 2, 3]", "stop", "JSON object"),
        ("```json\n{}\n```\n```json\n{}\n```", "stop", "not valid JSON"),
    ],
)
def test_non_json_responses(content: str | None, finish_reason: str, message: str) -> None:
    with pytest.raises(PlanError, match=message):
        _parse(content, finish_reason)


@pytest.mark.req("DEC-4")
@pytest.mark.parametrize(
    "fence",
    ["```json\n{}\n```", "```\n{}\n```", "  ```JSON\n{}\n```  \n", "```json\r\n{}\r\n```\r\n"],
)
def test_single_fenced_block_is_accepted(fence: str) -> None:
    plan = _parse(fence.replace("{}", _text(example_plan(IDS))))
    assert set(plan.stems) == set(IDS)


@pytest.mark.req("DEC-4")
def test_out_of_limits_and_nyquist_errors_are_listed() -> None:
    doc = example_plan(IDS)
    doc["stems"]["gtr"]["gain_db"] = 30.0
    doc["stems"]["bass"]["pan"] = "left"
    with pytest.raises(PlanError) as info:
        _parse(_text(doc))
    text = str(info.value)
    assert "stems.gtr.gain_db" in text
    assert "stems.bass.pan" in text
    doc = example_plan(IDS)
    doc["stems"]["gtr"]["eq"][1]["freq_hz"] = 9000
    with pytest.raises(PlanError, match="Nyquist"):
        parse_response(_text(doc), "stop", IDS, 16_000, iteration=1)


# --- change limits (DEC-5) ------------------------------------------------------------------


def _with(doc: dict[str, Any], sid: str, **values: Any) -> dict[str, Any]:
    doc = copy.deepcopy(doc)
    doc["stems"][sid].update(values)
    return doc


@pytest.mark.req("DEC-5")
@pytest.mark.parametrize(
    ("before", "after", "expected"),
    [(0.0, 10.0, 4.0), (0.0, -7.0, -4.0), (-20.0, 12.0, -16.0), (2.0, 5.5, 5.5), (1.5, -2.5, -2.5)],
)
def test_gain_change_is_clamped(before: float, after: float, expected: float) -> None:
    base = neutral_plan(IDS)
    prev = _plan(_with(base, "gtr", gain_db=before))
    new = _plan(_with(base, "gtr", gain_db=after))
    clamped, warnings = clamp_changes(prev, new)
    assert clamped.stems["gtr"].gain_db == expected  # exact: limits are exact decimals
    assert MAX_GAIN_CHANGE_DB == 4.0
    assert bool(warnings) == (expected != after)
    if warnings:
        assert "stems.gtr.gain_db" in warnings[0]


@pytest.mark.req("DEC-5")
def test_within_limits_is_identity() -> None:
    prev = _plan(example_plan(IDS))
    new_doc = example_plan(IDS)
    new_doc["stems"]["gtr"]["gain_db"] = 5.9
    new_doc["stems"]["gtr"]["eq"][0]["gain_db"] = 0.4
    new = _plan(new_doc)
    clamped, warnings = clamp_changes(prev, new)
    assert warnings == []
    assert clamped == new


@pytest.mark.req("DEC-5")
def test_first_iteration_has_no_limits() -> None:
    new = _plan(_with(neutral_plan(IDS), "gtr", gain_db=12.0))
    clamped, warnings = clamp_changes(None, new)
    assert clamped == new
    assert warnings == []


def _eq(*bands: tuple[str, float, float]) -> list[dict[str, Any]]:
    return [{"type": t, "freq_hz": f, "gain_db": g, "q": 1.0} for t, f, g in bands]


@pytest.mark.req("DEC-5")
def test_eq_change_is_clamped() -> None:
    base = neutral_plan(IDS)
    prev = _plan(_with(base, "gtr", eq=_eq(("peak", 300, -2.0), ("high_shelf", 9000, 1.0))))
    new = _plan(
        _with(
            base,
            "gtr",
            eq=_eq(
                ("peak", 450, -8.0),  # same position and type: -2 -> -8 limited to -5
                ("low_shelf", 100, 6.0),  # type changed: compared with 0 dB -> +3
                ("peak", 2500, -4.0),  # new band: compared with 0 dB -> -3
                ("peak", 5000, 2.5),  # new band within the limit: unchanged
            ),
        )
    )
    clamped, warnings = clamp_changes(prev, new)
    bands = clamped.stems["gtr"].eq_bands
    assert [b.gain_db for b in bands] == [-5.0, 3.0, -3.0, 2.5]
    assert [b.freq_hz for b in bands] == [450, 100, 2500, 5000]  # frequency is free
    assert MAX_EQ_CHANGE_DB == 3.0
    assert len(warnings) == 3
    assert all("stems.gtr.eq." in w for w in warnings)


@pytest.mark.req("DEC-5")
def test_removing_bands_is_allowed() -> None:
    base = neutral_plan(IDS)
    prev = _plan(_with(base, "gtr", eq=_eq(("peak", 300, -9.0), ("peak", 900, 9.0))))
    new = _plan(_with(base, "gtr", eq=None))
    clamped, warnings = clamp_changes(prev, new)
    assert clamped.stems["gtr"].eq is None
    assert warnings == []


@pytest.mark.req("DEC-5")
def test_clamped_plan_is_valid() -> None:
    prev = _plan(_with(neutral_plan(IDS), "gtr", gain_db=-24.0, eq=_eq(("peak", 300, -9.0))))
    new = _plan(_with(neutral_plan(IDS), "gtr", gain_db=12.0, eq=_eq(("peak", 300, 9.0))))
    clamped, _ = clamp_changes(prev, new)
    MixPlan.model_validate(clamped.model_dump(mode="json"))
    check_plan_fits(clamped, IDS, FS)
    assert clamped.stems["gtr"].gain_db == -20.0
    assert clamped.stems["gtr"].eq_bands[0].gain_db == -6.0


@pytest.mark.req("DEC-5")
def test_finalize_imposes_iteration_and_limits() -> None:
    prev = _plan(neutral_plan(IDS))
    inp = make_input(iteration=1, previous_plan=prev)
    proposed = _plan(_with(neutral_plan(IDS), "lead_vox", gain_db=9.0))
    decision = finalize(Decision(plan=proposed, warnings=("earlier",)), inp)
    assert decision.plan.iteration == 1
    assert decision.plan.stems["lead_vox"].gain_db == 4.0
    assert decision.warnings[0] == "earlier"
    assert "stems.lead_vox.gain_db" in decision.warnings[1]


@pytest.mark.req("DEC-5")
def test_clamped_values_have_no_float_artifacts() -> None:
    prev = _plan(_with(neutral_plan(IDS), "gtr", gain_db=-19.9, eq=_eq(("peak", 300, -2.2))))
    new = _plan(_with(neutral_plan(IDS), "gtr", gain_db=-10.0, eq=_eq(("peak", 300, 4.0))))
    clamped, _ = clamp_changes(prev, new)
    assert repr(clamped.stems["gtr"].gain_db) == "-15.9"
    assert repr(clamped.stems["gtr"].eq_bands[0].gain_db) == "0.8"


def test_finalize_rejects_a_plan_that_does_not_fit() -> None:
    """Whatever the decider, a plan outside the project (here: a missing stem) is refused."""
    inp = make_input()
    wrong = _plan(neutral_plan(["kick", "bass", "gtr"]))
    with pytest.raises(PlanError, match="stems missing from the plan: lead_vox"):
        finalize(Decision(plan=wrong), inp)
