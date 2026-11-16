"""From the model's answer to a plan the engine accepts (DEC-2, DEC-3, DEC-4, DEC-5).

`parse_response` turns the text of an answer into a validated `MixPlan` or raises `PlanError`
with every problem listed, so the message can be sent back to the model (DEC-4, D-040).
`clamp_changes` applies the change limits between iterations (DEC-5, D-042); `finalize` is the
last step of every decider: it imposes the iteration and the change limits.
"""

from __future__ import annotations

import dataclasses
import logging
import re

from stemwise.decision.base import Decision, DecisionInput
from stemwise.decision.schema import EqBand, MixPlan, check_plan_fits, dumps_plan, parse_plan
from stemwise.errors import PlanError

log = logging.getLogger(__name__)

MAX_GAIN_CHANGE_DB = 4.0
MAX_EQ_CHANGE_DB = 3.0

# One fenced block and nothing else (D-039): ```json ... ``` or ``` ... ```.
_FENCE = re.compile(r"^```(?:json)?[ \t]*\n(.*)\n[ \t]*```$", re.DOTALL | re.IGNORECASE)


def _strip_fence(text: str) -> str:
    match = _FENCE.match(text.replace("\r\n", "\n").strip())
    return match.group(1) if match else text


def _blank_texts(plan: MixPlan) -> list[str]:
    """DEC-3: summary and every rationale must say something, not just be non-empty."""
    blank = ["summary"] if not plan.summary.strip() else []
    blank += [
        f"stems.{sid}.rationale" for sid, sp in plan.stems.items() if not sp.rationale.strip()
    ]
    return blank


def parse_response(
    content: str | None,
    finish_reason: str | None,
    stem_ids: list[str],
    sample_rate: int,
    *,
    iteration: int,
) -> MixPlan:
    """Validate the text of an answer against the contract and the project; raise `PlanError`."""
    if finish_reason == "length":
        raise PlanError(
            "the response was truncated (finish_reason: length): answer with the complete plan only"
        )
    if content is None or not content.strip():
        raise PlanError("the response is empty: answer with the complete mix plan as JSON")
    plan = parse_plan(_strip_fence(content), "response")
    blank = _blank_texts(plan)
    if blank:
        raise PlanError(
            "invalid mix plan (response):\n"
            + "\n".join(f"  - {loc}: must contain text, not only spaces" for loc in blank)
        )
    check_plan_fits(plan, stem_ids, sample_rate)
    return plan.model_copy(update={"iteration": iteration})


def _limit(value: float, reference: float, max_change: float) -> float:
    """Clamp to reference ± max_change; rounded to 6 decimals (no -15.899999999999999)."""
    limited = min(reference + max_change, max(reference - max_change, value))
    return value if limited == value else round(limited, 6) + 0.0


def _clamp_eq(sid: str, prev: list[EqBand], new: list[EqBand], warnings: list[str]) -> list[EqBand]:
    """Bands are matched by position; a new band or a different type starts from 0 dB."""
    out: list[EqBand] = []
    for i, band in enumerate(new):
        ref = prev[i].gain_db if i < len(prev) and prev[i].type == band.type else 0.0
        gain = _limit(band.gain_db, ref, MAX_EQ_CHANGE_DB)
        if gain != band.gain_db:
            warnings.append(
                f"stems.{sid}.eq.{i}.gain_db: {band.gain_db:+.1f} dB is more than "
                f"{MAX_EQ_CHANGE_DB:g} dB away from the previous iteration ({ref:+.1f} dB): "
                f"set to {gain:+.1f} dB"
            )
            band = band.model_copy(update={"gain_db": gain})
        out.append(band)
    return out


def clamp_changes(previous: MixPlan | None, plan: MixPlan) -> tuple[MixPlan, list[str]]:
    """DEC-5: limit the change of stem gains (±4 dB) and EQ band gains (±3 dB)."""
    if previous is None:
        return plan, []
    warnings: list[str] = []
    stems = {}
    for sid, sp in plan.stems.items():
        before = previous.stems.get(sid)
        if before is None:
            stems[sid] = sp
            continue
        updates: dict[str, object] = {}
        gain = _limit(sp.gain_db, before.gain_db, MAX_GAIN_CHANGE_DB)
        if gain != sp.gain_db:
            warnings.append(
                f"stems.{sid}.gain_db: {sp.gain_db:+.1f} dB is more than "
                f"{MAX_GAIN_CHANGE_DB:g} dB away from the previous iteration "
                f"({before.gain_db:+.1f} dB): set to {gain:+.1f} dB"
            )
            updates["gain_db"] = gain
        if sp.eq:
            eq = _clamp_eq(sid, before.eq_bands, sp.eq, warnings)
            if eq != sp.eq:
                updates["eq"] = eq
        stems[sid] = sp.model_copy(update=updates) if updates else sp
    if not warnings:
        return plan, []
    return plan.model_copy(update={"stems": stems}), warnings


def finalize(decision: Decision, decision_input: DecisionInput) -> Decision:
    """Impose the iteration and the change limits on a decider's plan (D-039, DEC-5)."""
    plan = decision.plan.model_copy(update={"iteration": decision_input.iteration})
    # Whatever the decider, the engine accepts only plans within the contract and the project.
    plan = parse_plan(dumps_plan(plan), "decider plan")
    check_plan_fits(plan, decision_input.stem_ids, decision_input.sample_rate)
    plan, warnings = clamp_changes(decision_input.previous_plan, plan)
    for w in warnings:
        log.warning("change limit: %s", w)
    return dataclasses.replace(decision, plan=plan, warnings=(*decision.warnings, *warnings))
