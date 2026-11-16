"""Mock decider (DEC-7, DEC-3, D-043): deterministic, offline, always a valid plan."""

from __future__ import annotations

import dataclasses
import math

import pytest

from stemwise.decision.base import Decider, HistoryEntry
from stemwise.decision.mock import (
    DRUM_ROLES,
    EQ_CUT_DB,
    MAX_EQ_CUTS,
    ROLE_GAIN_DB,
    MockDecider,
)
from stemwise.decision.schema import MixPlan, check_plan_fits
from tests.helpers.decision import ALL_ROLES, make_input

pytestmark = pytest.mark.req("DEC-7")


def _plan(**kwargs) -> MixPlan:
    return MockDecider().decide(make_input(**kwargs)).plan


def test_is_a_decider() -> None:
    assert isinstance(MockDecider(), Decider)
    assert MockDecider().name == "mock"


def test_deterministic() -> None:
    inp = make_input(collisions=[("gtr", "lead_vox", "2k-4k", 0.4)])
    first = MockDecider().decide(inp).plan.model_dump()
    second = MockDecider().decide(inp).plan.model_dump()
    assert first == second


@pytest.mark.parametrize("sample_rate", [44_100, 48_000, 16_000])
@pytest.mark.parametrize("n", [2, 7, 16])
def test_valid_for_any_project(n: int, sample_rate: int) -> None:
    stems = [(f"s{i}", ALL_ROLES[i % len(ALL_ROLES)], 1 + i % 2) for i in range(n)]
    collisions = [
        (stems[i][0], stems[(i + 1) % n][0], band, 0.5)
        for i, band in zip(
            range(min(n, 10)), ["8k-20k", "4k-8k", "500-2k", "60-120"] * 3, strict=False
        )
    ]
    plan = _plan(stems=stems, sample_rate=sample_rate, collisions=collisions)
    MixPlan.model_validate(plan.model_dump(mode="json"))  # within the limits of PRD §6.3
    check_plan_fits(plan, [s[0] for s in stems], sample_rate)  # ids and Nyquist


@pytest.mark.req("DEC-3")
def test_every_stem_has_rationale() -> None:
    stems = [(f"s{i}", role, 1) for i, role in enumerate(ALL_ROLES)]
    plan = _plan(stems=stems)
    assert plan.summary.strip()
    assert all(sp.rationale.strip() for sp in plan.stems.values())


def test_gain_by_role_and_vocal_in_front() -> None:
    plan = _plan()
    assert plan.stems["lead_vox"].gain_db == ROLE_GAIN_DB["lead_vocal"] == 2.0
    assert plan.stems["lead_vox"].gain_db > max(
        sp.gain_db for sid, sp in plan.stems.items() if sid != "lead_vox"
    )


def test_drum_family_compensated_for_number_of_stems() -> None:
    drums = [("kick", "kick", 1), ("snare", "snare", 1), ("oh", "overheads", 2)]
    plan = _plan(stems=[*drums, ("bass", "bass", 1), ("vox", "lead_vocal", 1)])
    offset = -10.0 * math.log10(len(drums))  # 3 stems of one instrument: -4.8 dB each
    for sid, role, _ in drums:
        assert role in DRUM_ROLES
        assert plan.stems[sid].gain_db == pytest.approx(ROLE_GAIN_DB[role] + offset, abs=0.05)
    assert plan.stems["bass"].gain_db == ROLE_GAIN_DB["bass"]  # not a drum: unchanged


def test_single_drum_stem_not_compensated() -> None:
    plan = _plan(stems=[("kit", "drums", 2), ("vox", "lead_vocal", 1)])
    assert plan.stems["kit"].gain_db == ROLE_GAIN_DB["drums"]


def test_pan_rules() -> None:
    stems = [
        ("kick", "kick", 1),
        ("gtr_a", "electric_guitar", 1),
        ("gtr_b", "electric_guitar", 1),
        ("gtr_c", "electric_guitar", 1),
        ("keys", "keys", 1),
        ("pad", "pad", 2),
        ("vox", "lead_vocal", 1),
    ]
    plan = _plan(stems=stems)
    assert plan.stems["kick"].pan == 0.0
    assert plan.stems["vox"].pan == 0.0
    assert [plan.stems[s].pan for s in ("gtr_a", "gtr_b", "gtr_c")] == [-0.5, 0.5, -0.5]
    assert plan.stems["keys"].pan == 0.0  # the only mono stem of its role
    assert plan.stems["pad"].pan == 0.0  # stereo


def test_eq_cut_on_the_stem_that_masks_the_vocal() -> None:
    plan = _plan(collisions=[("gtr", "lead_vox", "2k-4k", 0.4), ("kick", "bass", "60-120", 0.3)])
    gtr = plan.stems["gtr"].eq_bands
    assert len(gtr) == 1
    assert gtr[0].type == "peak"
    assert gtr[0].gain_db == EQ_CUT_DB == -2.0
    assert gtr[0].freq_hz == pytest.approx(math.sqrt(2000 * 4000), abs=1)
    assert plan.stems["lead_vox"].eq_bands == []
    assert len(plan.stems["kick"].eq_bands) == 1  # kick yields to bass (lower priority)
    assert plan.stems["bass"].eq_bands == []


def test_eq_ignores_weak_and_unrelated_collisions() -> None:
    plan = _plan(
        stems=[("gtr", "electric_guitar", 2), ("keys", "keys", 1), ("vox", "lead_vocal", 1)],
        collisions=[("gtr", "keys", "500-2k", 0.6), ("gtr", "vox", "2k-4k", 0.01)],
    )
    assert all(sp.eq_bands == [] for sp in plan.stems.values())


def test_at_most_two_cuts_per_stem() -> None:
    bands = ["120-250", "250-500", "500-2k", "2k-4k"]
    plan = _plan(collisions=[("gtr", "lead_vox", b, 0.5 - i * 0.05) for i, b in enumerate(bands)])
    gtr = plan.stems["gtr"].eq_bands
    assert len(gtr) == MAX_EQ_CUTS == 2
    assert [round(b.freq_hz) for b in gtr] == [
        round(math.sqrt(120 * 250)),
        round(math.sqrt(250 * 500)),
    ]


def test_request_without_previous_plan_starts_from_the_rules() -> None:
    plan = _plan(request="vocal louder")
    base = _plan()
    assert plan.stems["lead_vox"].gain_db == base.stems["lead_vox"].gain_db + 2.0
    assert {s: p for s, p in plan.stems.items() if s != "lead_vox"} == {
        s: p for s, p in base.stems.items() if s != "lead_vox"
    }


def test_reverb_present_only_with_sends() -> None:
    assert _plan().reverb is not None
    no_sends = _plan(stems=[("kick", "kick", 1), ("bass", "bass", 1)])
    assert no_sends.reverb is None
    assert all(sp.reverb_send_db is None for sp in no_sends.stems.values())


def test_logs_the_messages_a_real_decider_would_send() -> None:
    from stemwise.decision.prompt import build_messages

    inp = make_input()
    log = MockDecider().decide(inp).request_log
    assert log == {"decider": "mock", "sent": False, "messages": build_messages(inp)}


# --- feedback from the evaluation (D-057) --------------------------------------------------------


def _with_history(deltas: list[float] | None, previous: MixPlan, *, shares=None):
    inp = make_input(iteration=1, previous_plan=previous)
    if shares is not None:
        analysis = inp.analysis.model_copy(update={"band_share_pct": shares})
        inp = dataclasses.replace(inp, analysis=analysis)
    evaluation = {"band_delta_db": deltas, "spectral_distance_db": 2.0}
    return dataclasses.replace(inp, history=(HistoryEntry(previous, evaluation),))


def _shares(winner: str, band: int) -> dict[str, list[float]]:
    ids = ["kick", "bass", "gtr", "lead_vox"]
    shares = {sid: [25.0] * 8 for sid in ids}
    shares[winner][band] = 40.0
    return shares


def test_feedback_rule() -> None:
    first = _plan()
    deltas = [0.5, -1.0, 0.0, 0.2, 0.3, 5.0, -2.0, 0.0]  # largest: +5 dB in 2k-4k
    inp = _with_history(deltas, first, shares=_shares("gtr", 5))
    plan = MockDecider().decide(inp).plan
    added = plan.stems["gtr"].eq[-1]
    assert (added.type, added.freq_hz, added.gain_db, added.q) == ("peak", 2828.0, 3.0, 1.0)
    assert "2k-4k" in plan.stems["gtr"].rationale
    assert plan.iteration == 1
    others = {sid: sp for sid, sp in plan.stems.items() if sid != "gtr"}
    assert others == {sid: sp for sid, sp in first.stems.items() if sid != "gtr"}
    assert MockDecider().decide(inp).plan == plan  # deterministic


def test_feedback_adjusts_the_existing_peak() -> None:
    first = _plan(collisions=[("gtr", "lead_vox", "2k-4k", 0.4)])
    assert first.stems["gtr"].eq[0].gain_db == EQ_CUT_DB  # the -2 dB cut at 2828 Hz
    deltas = [0.0, 0.0, 0.0, 0.0, 0.0, 2.5, 0.0, 0.0]
    plan = MockDecider().decide(_with_history(deltas, first, shares=_shares("gtr", 5))).plan
    assert len(plan.stems["gtr"].eq) == len(first.stems["gtr"].eq)
    assert plan.stems["gtr"].eq[0].gain_db == 0.5


@pytest.mark.parametrize("deltas", [None, [0.5, -0.9, 0.0, 0.2, 0.3, 0.4, -0.2, 0.0]])
def test_no_feedback_without_target_or_small_differences(deltas) -> None:
    first = _plan()
    plan = MockDecider().decide(_with_history(deltas, first)).plan
    assert plan.stems == first.stems
    assert plan.iteration == 1


# --- the author's request (REV-1, D-065) ---------------------------------------------------------

REQUEST_STEMS = [
    ("kick", "kick", 1),
    ("snare", "snare", 1),
    ("bass", "bass", 1),
    ("gtr_l", "electric_guitar", 1),
    ("gtr_r", "electric_guitar", 1),
    ("bvox", "backing_vocal", 1),
    ("lead_vox", "lead_vocal", 1),
]


def _revise(request: str) -> tuple[MixPlan, MixPlan]:
    start = _plan(stems=REQUEST_STEMS)
    inp = make_input(stems=REQUEST_STEMS, request=request, previous_plan=start)
    return start, MockDecider().decide(inp).plan


def _changes(start: MixPlan, plan: MixPlan) -> dict[str, dict[str, float | None]]:
    out: dict[str, dict[str, float | None]] = {}
    for sid, sp in plan.stems.items():
        before = start.stems[sid]
        for field in ("gain_db", "reverb_send_db"):
            if getattr(sp, field) != getattr(before, field):
                out.setdefault(sid, {})[field] = getattr(sp, field)
    return out


@pytest.mark.req("REV-1")
@pytest.mark.parametrize(
    ("request_", "expected"),
    [
        ("voce più avanti", {"lead_vox": {"gain_db": 4.0}}),
        ("Lead vocal louder", {"lead_vox": {"gain_db": 4.0}}),
        ("alza il basso", {"bass": {"gain_db": 2.0}}),
        ("backing vocals down", {"bvox": {"gain_db": -6.0}}),
        ("batteria più indietro", {"kick": {"gain_db": -5.0}, "snare": {"gain_db": -5.0}}),
        ("gtr_l quieter", {"gtr_l": {"gain_db": -4.0}}),
        (
            "less reverb on the guitars",
            {"gtr_l": {"reverb_send_db": -27.0}, "gtr_r": {"reverb_send_db": -27.0}},
        ),
        ("more reverb on the kick", {"kick": {"reverb_send_db": -24.0}}),
        (
            "meno riverbero",
            {
                "snare": {"reverb_send_db": -29.0},
                "gtr_l": {"reverb_send_db": -27.0},
                "gtr_r": {"reverb_send_db": -27.0},
                "bvox": {"reverb_send_db": -21.0},
                "lead_vox": {"reverb_send_db": -23.0},
            },
        ),
        (
            "voce più avanti, meno riverbero sulle chitarre",
            {
                "gtr_l": {"reverb_send_db": -27.0},
                "gtr_r": {"reverb_send_db": -27.0},
                "lead_vox": {"gain_db": 4.0},
            },
        ),
    ],
)
def test_request_rules(request_: str, expected: dict) -> None:
    start, plan = _revise(request_)
    assert _changes(start, plan) == expected
    assert plan.reverb == start.reverb
    for sid in expected:
        assert "Revision" in plan.stems[sid].rationale
    assert plan.summary.startswith("Mock revision")
    assert _revise(request_)[1] == plan  # deterministic


@pytest.mark.req("REV-1")
@pytest.mark.parametrize("request_", ["make it sound like the eighties", "voce", "più avanti"])
def test_request_not_understood_keeps_the_plan(request_: str) -> None:
    start, plan = _revise(request_)
    assert plan.stems == start.stems
    assert "not understood" in plan.summary


@pytest.mark.req("REV-1")
def test_request_at_the_limit_says_so() -> None:
    start = _plan(stems=REQUEST_STEMS)
    doc = start.model_dump(mode="json")
    doc["stems"]["lead_vox"]["gain_db"] = 12.0  # the maximum of the plan
    at_limit = MixPlan.model_validate(doc)
    inp = make_input(stems=REQUEST_STEMS, request="voce più avanti", previous_plan=at_limit)
    plan = MockDecider().decide(inp).plan
    assert plan.stems == at_limit.stems
    assert "lead_vox gain already at the limit" in plan.summary


@pytest.mark.req("REV-1")
def test_lead_guitar_is_not_the_vocal() -> None:
    start, plan = _revise("lead guitar louder")
    assert plan.stems["lead_vox"].gain_db == start.stems["lead_vox"].gain_db
    assert plan.stems["gtr_l"].gain_db == start.stems["gtr_l"].gain_db + 2.0
