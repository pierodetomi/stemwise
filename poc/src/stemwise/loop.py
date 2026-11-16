"""The verification loop: decision → render → evaluation, up to 3 times (PRD §4, EVA-5, EVA-6).

`stop_reason` and `select_best` are pure functions of the evaluations written so far (D-051,
D-052). `run_loop` drives the iterations with the decider, the render and the evaluation
injected, so it holds no audio and writes no file: the caller does that in `on_iteration`.
Decisions use the values as written in `evaluation.json` (one decimal), compared in integer
tenths, so they can be re-derived from the files of the run.
"""

from __future__ import annotations

import dataclasses
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from typing import Any, Generic, TypeVar

from stemwise.decision.base import Decider, Decision, DecisionInput, HistoryEntry
from stemwise.decision.validate import finalize
from stemwise.errors import DecisionError
from stemwise.evaluate import Best, Evaluation, StopReason

DISTANCE_THRESHOLD_DB = 1.5  # EVA-5: stop when the distance falls below this
MIN_IMPROVEMENT_DB = 0.2  # EVA-5: stop when the distance improves by less than this
DEFAULT_MAX_ITERATIONS = 3

MixT = TypeVar("MixT")


def _tenths(value: float) -> int:
    return round(value * 10)


def stop_reason(evaluations: Sequence[Evaluation], max_iterations: int) -> StopReason | None:
    """EVA-5 (D-051, D-052): why the loop stops after the last evaluation, or None to go on.

    In order: the distance is below the threshold (only for an iteration within the constraints);
    it improved by less than the minimum on the previous iteration (getting worse included;
    iteration 0 is not compared with the raw sum); the maximum is reached. Without a target the
    loop stops at the first iteration within the constraints.
    """
    current = evaluations[-1]
    distance = current.spectral_distance_db
    valid = current.constraints.all_ok
    if distance is None:
        if valid:
            return "no_target"
    else:
        if valid and _tenths(distance) < _tenths(DISTANCE_THRESHOLD_DB):
            return "below_threshold"
        previous = evaluations[-2].spectral_distance_db if len(evaluations) > 1 else None
        if previous is not None and _tenths(previous) - _tenths(distance) < _tenths(
            MIN_IMPROVEMENT_DB
        ):
            return "no_improvement"
    if len(evaluations) >= max_iterations:
        return "max_iterations"
    return None


def revision_stop(evaluations: Sequence[Evaluation], max_iterations: int) -> StopReason:
    """REV-1, REV-2 (D-060): a revision is one iteration, never chosen by distance."""
    return "revision"


def select_best(evaluations: Sequence[Evaluation]) -> int | None:
    """EVA-6 (D-051): index of the minimum distance among the iterations within the constraints.

    Ties go to the oldest iteration; without a target (no distance) the first valid one wins.
    None if no iteration is within the constraints.
    """
    valid = [i for i, e in enumerate(evaluations) if e.constraints.all_ok]
    if not valid:
        return None

    def key(i: int) -> tuple[int, int, int]:
        distance = evaluations[i].spectral_distance_db
        return (1, 0, i) if distance is None else (0, _tenths(distance), i)

    return min(valid, key=key)


def plan_path(iteration: int) -> str:
    """Path of the plan of an iteration, relative to the run folder (PRD §8)."""
    return f"iter_{iteration:02d}/plan.json"


def best_of(evaluations: Sequence[Evaluation], stop: StopReason | None) -> Best:
    index = select_best(evaluations)
    if index is None:
        return Best(iteration=None, plan=None, spectral_distance_db=None, stop_reason=stop)
    chosen = evaluations[index]
    return Best(
        iteration=chosen.iteration,
        plan=plan_path(index),
        spectral_distance_db=chosen.spectral_distance_db,
        stop_reason=stop,
    )


@dataclass(frozen=True)
class Iteration(Generic[MixT]):
    iteration: int
    decision: Decision
    mix: MixT
    evaluation: Evaluation  # with `is_best` and `stop_reason` set


@dataclass(frozen=True)
class LoopResult(Generic[MixT]):
    iterations: list[Iteration[MixT]]
    best: Best
    failure: DecisionError | None = None  # the decision that stopped the loop (D-055)
    failed_iteration: int | None = None


def run_loop(
    decider: Decider,
    first_input: DecisionInput,
    *,
    render: Callable[[Any], MixT],
    evaluate: Callable[[MixT, int], Evaluation],
    max_iterations: int = DEFAULT_MAX_ITERATIONS,
    on_decision: Callable[[int, Decision], None] | None = None,
    on_iteration: Callable[[Iteration[MixT], Best], None] | None = None,
    stop_rule: Callable[[Sequence[Evaluation], int], StopReason | None] = stop_reason,
) -> LoopResult[MixT]:
    """Run up to `max_iterations` iterations from `first_input` (iteration 0).

    The history and the previous plan of `first_input` are the starting point: empty for `mix`,
    the starting plan with its evaluation for a revision (REV-1, D-062), which also passes
    `stop_rule=revision_stop` (one iteration, best if within the constraints, REV-2).

    Each decision sees the previous iterations with their evaluations (DEC-1, the prompt keeps
    the last 2) and is limited by DEC-5 against the previous plan (`finalize`). A decision that
    fails on iteration 0 raises; on a later iteration it stops the loop and the iterations done
    are kept (D-055). `on_decision` receives each decision before its render (so the logs of a
    paid call are written even if the render fails); `on_iteration` receives each evaluated
    iteration and the best so far, so the caller can write `best.json` after every iteration.
    """
    if max_iterations < 1:
        raise ValueError("the loop needs at least one iteration")
    iterations: list[Iteration[MixT]] = []
    evaluations: list[Evaluation] = []
    history: list[HistoryEntry] = list(first_input.history)
    previous_plan = first_input.previous_plan
    stop: StopReason | None = None
    for k in range(max_iterations):
        inp = dataclasses.replace(
            first_input, iteration=k, history=tuple(history), previous_plan=previous_plan
        )
        try:
            decision = finalize(decider.decide(inp), inp)
        except DecisionError as exc:
            if k == 0:
                raise
            stop = "decision_failed"
            return LoopResult(iterations, best_of(evaluations, stop), exc, k)
        if on_decision is not None:
            on_decision(k, decision)
        mix = render(decision.plan)
        raw = evaluate(mix, k)
        evaluations.append(raw)
        stop = stop_rule(evaluations, max_iterations)
        evaluation = raw.model_copy(
            update={"is_best": select_best(evaluations) == k, "stop_reason": stop}
        )
        evaluations[-1] = evaluation
        iteration = Iteration(k, decision, mix, evaluation)
        iterations.append(iteration)
        if on_iteration is not None:
            on_iteration(iteration, best_of(evaluations, stop))
        if stop is not None:
            break
        history.append(HistoryEntry(decision.plan, evaluation.model_dump(mode="json")))
        previous_plan = decision.plan
    return LoopResult(iterations, best_of(evaluations, stop))
