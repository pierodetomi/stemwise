"""Verification loop: stop criteria (EVA-5), best plan (EVA-6), history and change limits."""

from __future__ import annotations

from typing import Any

import pytest

from stemwise.decision.base import Decision, DecisionInput, HistoryEntry
from stemwise.decision.schema import MixPlan
from stemwise.errors import DecisionError
from stemwise.evaluate import Evaluation
from stemwise.loop import revision_stop, run_loop, select_best, stop_reason
from tests.helpers.decision import make_input
from tests.helpers.plan import neutral_plan

IDS = ["kick", "bass", "gtr", "lead_vox"]

MIX = {
    "lufs": -18.0,
    "peak_db": -3.5,
    "true_peak_db": -3.0,
    "rms_db": -17.0,
    "crest_db": 13.5,
    "dyn_range_lu": 7.0,
    "bands_db": [-30.0, -20.0, -12.0, -8.0, -5.0, -10.0, -15.0, -22.0],
    "stereo": {"correlation": 0.8, "side_mid_db": -11.0},
    "low_corr": 0.95,
}


def ev(distance: float | None, *, ok: bool = True, iteration: int = 0) -> Evaluation:
    return Evaluation.model_validate(
        {
            "iteration": iteration,
            "spectral_distance_db": distance,
            "band_delta_db": None if distance is None else [0.0] * 8,
            "width_delta_db": None,
            "dyn_delta_lu": None,
            "masking_top": [],
            "constraints": {"true_peak_ok": True, "low_corr_ok": ok, "dc_ok": True},
            "is_best": False,
            "stop_reason": None,
            "mix": MIX,
        }
    )


@pytest.mark.req("EVA-5")
@pytest.mark.parametrize(
    ("distances", "max_iter", "expected"),
    [
        ([1.4], 3, "below_threshold"),
        ([1.5], 3, None),  # "below 1.5": 1.5 itself continues
        ([2.0], 3, None),
        ([3.0, 2.9], 3, "no_improvement"),
        ([3.0, 2.8], 3, None),  # 0.2 is not "less than 0.2" (tenths, D-051)
        ([3.0, 3.5], 3, "no_improvement"),  # getting worse
        ([3.0, 2.5, 2.0], 3, "max_iterations"),
        ([3.0], 1, "max_iterations"),
        ([3.0, 1.4], 3, "below_threshold"),  # threshold first
        ([1.4, 1.3], 3, "below_threshold"),
    ],
)
def test_stop_reason(distances: list[float], max_iter: int, expected: str | None) -> None:
    evaluations = [ev(d, iteration=i) for i, d in enumerate(distances)]
    assert stop_reason(evaluations, max_iter) == expected


@pytest.mark.req("EVA-5")
def test_violated_constraints_do_not_stop_on_the_threshold() -> None:
    assert stop_reason([ev(1.2, ok=False)], 3) is None
    assert stop_reason([ev(1.2, ok=False)], 1) == "max_iterations"


@pytest.mark.req("EVA-5")
def test_without_target_stops_at_the_first_valid_iteration() -> None:
    assert stop_reason([ev(None)], 3) == "no_target"
    assert stop_reason([ev(None, ok=False)], 3) is None
    assert stop_reason([ev(None, ok=False), ev(None)], 3) == "no_target"


@pytest.mark.req("EVA-6")
@pytest.mark.parametrize(
    ("evaluations", "expected"),
    [
        ([ev(3.0), ev(2.0), ev(2.5)], 1),
        ([ev(3.0), ev(1.0, ok=False), ev(2.5)], 2),  # the lowest one violates a constraint
        ([ev(2.0), ev(2.0)], 0),  # tie: the oldest
        ([ev(2.04), ev(2.0)], 0),  # equal as written (one decimal)
        ([ev(1.0, ok=False)], None),
        ([ev(None, ok=False), ev(None), ev(None)], 1),  # no target: the first valid
    ],
)
def test_select_best(evaluations: list[Evaluation], expected: int | None) -> None:
    assert select_best(evaluations) == expected


# --- the loop driver -----------------------------------------------------------------------------


def _plan(**stem: Any) -> MixPlan:
    return MixPlan.model_validate(neutral_plan(IDS, **stem))


class Scripted:
    """A decider that returns the given plans in turn (or raises) and records its inputs."""

    name = "scripted"

    def __init__(self, *answers: MixPlan | Exception) -> None:
        self.answers = list(answers)
        self.inputs: list[DecisionInput] = []

    def decide(self, decision_input: DecisionInput) -> Decision:
        self.inputs.append(decision_input)
        answer = self.answers.pop(0)
        if isinstance(answer, Exception):
            raise answer
        return Decision(plan=answer, request_log={"decider": self.name})


def _run(decider: Scripted, distances: list[float | None], max_iter: int = 3, **kw: Any):
    seen: list[tuple[int, int | None]] = []
    rendered: list[MixPlan] = []

    def render(plan: MixPlan) -> MixPlan:
        rendered.append(plan)
        return plan

    def evaluate(_mix: object, iteration: int) -> Evaluation:
        return ev(distances[iteration], iteration=iteration)

    result = run_loop(
        decider,
        make_input(),
        render=render,
        evaluate=evaluate,
        max_iterations=max_iter,
        on_iteration=lambda it, best: seen.append((it.iteration, best.iteration)),
        **kw,
    )
    return result, seen, rendered


@pytest.mark.req("EVA-6")
def test_loop_tracks_best() -> None:
    decider = Scripted(_plan(gain_db=0.0), _plan(gain_db=1.0), _plan(gain_db=2.0))
    result, seen, _ = _run(decider, [3.0, 2.0, 2.5])
    flags = [it.evaluation.is_best for it in result.iterations]
    assert flags == [True, True, False]
    assert [it.evaluation.stop_reason for it in result.iterations] == [None, None, "no_improvement"]
    assert result.best.iteration == 1
    assert result.best.plan == "iter_01/plan.json"
    assert result.best.spectral_distance_db == 2.0
    assert result.best.stop_reason == "no_improvement"
    assert seen == [(0, 0), (1, 1), (2, 1)]  # best.json can be written after every iteration


@pytest.mark.req("EVA-5")
def test_loop_stops_below_threshold() -> None:
    result, _, _ = _run(Scripted(_plan(), _plan()), [1.4, 1.0])
    assert len(result.iterations) == 1
    assert result.best.stop_reason == "below_threshold"


@pytest.mark.req("DEC-1", "DEC-5")
def test_history_and_change_limits_in_the_loop() -> None:
    decider = Scripted(_plan(gain_db=0.0), _plan(gain_db=10.0), _plan(gain_db=-2.0))
    result, _, rendered = _run(decider, [3.0, 2.0, 1.0])
    first, second, third = decider.inputs
    assert [i.iteration for i in decider.inputs] == [0, 1, 2]
    assert first.history == ()
    assert first.previous_plan is None
    assert second.previous_plan == rendered[0]
    assert len(third.history) == 2
    assert third.history[0].plan == rendered[0]
    assert third.history[1].evaluation["spectral_distance_db"] == 2.0
    assert third.history[1].evaluation["is_best"] is True
    assert rendered[1].stems["kick"].gain_db == 4.0  # +10 dB clipped to +4 (DEC-5)
    assert any("gain_db" in w for w in result.iterations[1].decision.warnings)
    assert rendered[1].iteration == 1


@pytest.mark.req("EVA-5")
def test_history_keeps_only_the_last_two() -> None:
    decider = Scripted(_plan(), _plan(gain_db=1.0), _plan(gain_db=2.0), _plan(gain_db=3.0))
    _run(decider, [5.0, 4.0, 3.0, 2.0], max_iter=4)
    assert len(decider.inputs[3].history) == 3  # the prompt keeps the last 2 (DEC-1)
    assert len(decider.inputs[3].recent_history) == 2


def test_decision_failure_on_the_first_iteration_raises() -> None:
    with pytest.raises(DecisionError, match="boom"):
        _run(Scripted(DecisionError("boom")), [3.0])


@pytest.mark.req("EVA-6")
def test_decision_failure_later_keeps_the_result() -> None:
    failure = DecisionError("3 attempts")
    result, _, _ = _run(Scripted(_plan(), failure), [3.0, 2.0])
    assert len(result.iterations) == 1
    assert result.failure is failure
    assert result.failed_iteration == 1
    assert result.best.iteration == 0
    assert result.best.stop_reason == "decision_failed"


@pytest.mark.req("EVA-6")
def test_no_valid_iteration() -> None:
    decider = Scripted(_plan(), _plan(gain_db=1.0))

    def evaluate(_mix: object, iteration: int) -> Evaluation:
        return ev(3.0 - iteration, ok=False, iteration=iteration)

    result = run_loop(
        decider,
        make_input(),
        render=lambda plan: plan,
        evaluate=evaluate,
        max_iterations=2,
    )
    assert result.best.iteration is None
    assert result.best.plan is None
    assert result.best.stop_reason == "max_iterations"
    assert not any(it.evaluation.is_best for it in result.iterations)


# --- revision: one iteration from a starting plan (REV-1, REV-2, D-060, D-063) ------------------


@pytest.mark.req("REV-1", "REV-2")
def test_loop_starts_from_the_given_history() -> None:
    import dataclasses

    start = _plan(gain_db=1.0)
    entry = HistoryEntry(start, ev(2.0, iteration=1).model_dump(mode="json"))
    first = dataclasses.replace(
        make_input(), history=(entry,), previous_plan=start, request="voce più avanti"
    )
    decider = Scripted(_plan(gain_db=10.0))
    rendered: list[MixPlan] = []
    result = run_loop(
        decider,
        first,
        render=lambda plan: rendered.append(plan) or plan,
        evaluate=lambda _mix, k: ev(5.0, iteration=k),  # farther from the target than the start
        max_iterations=1,
        stop_rule=revision_stop,
    )
    (inp,) = decider.inputs
    assert inp.iteration == 0
    assert inp.history == (entry,)
    assert inp.previous_plan == start
    assert inp.request == "voce più avanti"
    assert rendered[0].stems["kick"].gain_db == 5.0  # +9 dB from the start clipped to +4 (DEC-5)
    assert result.best.iteration == 0  # no rollback (REV-2)
    assert result.best.stop_reason == "revision"
    assert result.iterations[0].evaluation.is_best is True
    assert result.iterations[0].evaluation.stop_reason == "revision"


@pytest.mark.req("REV-2")
def test_revision_with_violated_constraints_has_no_best() -> None:
    result = run_loop(
        Scripted(_plan()),
        make_input(),
        render=lambda plan: plan,
        evaluate=lambda _mix, k: ev(1.0, ok=False, iteration=k),
        max_iterations=1,
        stop_rule=revision_stop,
    )
    assert result.best.iteration is None
    assert result.best.stop_reason == "revision"
