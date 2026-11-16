"""Messages sent to the model (DEC-1, DEC-3, D-041).

The system prompt states the role, the processing chain, units and every parameter limit. The
limits are generated from the `MixPlan` model (never copied by hand), so they cannot drift from
the validation. The user message carries the brief, the stems (id, role, author's notes), the
analysis exactly as written to `analysis.json`, the last 2 iterations and the author's request.
No audio, no file paths, no song title ever enter the messages (PRD §7).
"""

from __future__ import annotations

import json
from typing import Any

from stemwise.decision.base import DecisionInput
from stemwise.decision.schema import MAX_EQ_BANDS, MixPlan
from stemwise.decision.validate import MAX_EQ_CHANGE_DB, MAX_GAIN_CHANGE_DB
from stemwise.loop import DISTANCE_THRESHOLD_DB, MIN_IMPROVEMENT_DB

_UNITS = {"db": "dB", "hz": "Hz", "ms": "ms"}


def _unit(name: str) -> str:
    return _UNITS.get(name.rsplit("_", 1)[-1], "")


def _resolve(schema: dict[str, Any], node: dict[str, Any]) -> tuple[dict[str, Any], bool]:
    """Return the concrete sub-schema of a property and whether `null` is allowed."""
    nullable = False
    if "anyOf" in node:
        options = [o for o in node["anyOf"] if o.get("type") != "null"]
        nullable = len(options) < len(node["anyOf"])
        node = options[0]
    if "$ref" in node:
        node = schema["$defs"][node["$ref"].rsplit("/", 1)[-1]]
    return node, nullable


def _rows(schema: dict[str, Any], prefix: str, node: dict[str, Any]) -> list[tuple[str, ...]]:
    rows: list[tuple[str, ...]] = []
    for name, prop in node.get("properties", {}).items():
        concrete, nullable = _resolve(schema, prop)
        path = f"{prefix}{name}"
        if concrete.get("type") == "array":
            item, _ = _resolve(schema, concrete["items"])
            rows.append((path, "0 bands", f"{concrete['maxItems']} bands", _null(nullable)))
            rows += _rows(schema, f"{path}[].", item)
        elif "properties" in concrete:
            rows.append((path, "", "", "object" + (", or null = off" if nullable else "")))
            rows += _rows(schema, f"{path}.", concrete)
        elif "minimum" in concrete or "maximum" in concrete:
            unit = _unit(name)
            lo = f"{concrete['minimum']:g} {unit}".strip() if "minimum" in concrete else "-"
            hi = f"{concrete['maximum']:g} {unit}".strip() if "maximum" in concrete else "-"
            rows.append((path, lo, hi, _null(nullable)))
        elif "enum" in concrete:
            rows.append((path, "", "", "one of: " + ", ".join(concrete["enum"])))
    return rows


def limit_rows() -> list[tuple[str, ...]]:
    """(parameter, minimum, maximum, notes) for every bounded field of the plan, from the model."""
    schema = MixPlan.model_json_schema()
    defs = schema["$defs"]
    return [
        *_rows(schema, "stems.<id>.", defs["StemPlan"]),
        ("reverb", "", "", "object, or null = no reverb"),
        *_rows(schema, "reverb.", defs["Reverb"]),
    ]


def _null(nullable: bool) -> str:
    return "null = off" if nullable else ""


def _limits_table() -> str:
    lines = ["| parameter | min | max | notes |", "| --- | --- | --- | --- |"]
    lines += [f"| {p} | {lo} | {hi} | {n} |" for p, lo, hi, n in limit_rows()]
    return "\n".join(lines)


def system_prompt(inp: DecisionInput) -> str:
    pre_gain = (
        "Before your plan is applied, every stem is normalised to -23 LUFS integrated (pre-gain), "
        "so `gain_db` 0 on two stems means equal loudness and gains are relative balances. "
        "The per-stem loudness in the analysis is measured before this normalisation."
        if inp.pre_normalize
        else "Stems are NOT normalised before your plan: `gain_db` acts on the raw recorded "
        "levels shown in the analysis."
    )
    return f"""\
You are a mixing engineer. You never hear the audio: you decide static mix parameters for the
whole song from numeric measurements of the stems, the author's brief and, when present, a
target profile measured on reference tracks.

## Processing chain
Each stem goes through, in this fixed order: pre-gain, high-pass (12 dB/octave), low-pass
(12 dB/octave), EQ bands in list order (RBJ biquads: peak, low_shelf, high_shelf), compressor,
makeup gain, `gain_db`, pan, send to one shared reverb (100% wet, post-fader, post-pan). The mix
bus only sums the stems and the reverb return, then trims the level so that the true peak stays
at or below {inp.true_peak_ceiling_db:g} dBTP; there is no limiter and no other bus processing.
{pre_gain}
Pan: -1 is hard left, +1 hard right; mono stems use a constant-power law, stereo stems a balance.

## Parameter limits
Every value must stay within these limits; `null` switches a block off. An empty `eq` list
also means no EQ (at most {MAX_EQ_BANDS} bands).
{_limits_table()}

Between one iteration and the next, `gain_db` of a stem may change by at most
±{MAX_GAIN_CHANGE_DB:g} dB and the gain of each EQ band (matched by position and type) by at most
±{MAX_EQ_CHANGE_DB:g} dB; larger changes are clipped by the engine.

## How to read the analysis
- `bands_db`: energy in 8 fixed bands (20-60, 60-120, 120-250, 250-500, 500-2k, 2k-4k, 4k-8k,
  8k-20k Hz), in dB relative to the stem total. `band_share_pct`: share of each stem in the mix
  energy per band. `target`: mean profile of the references, if any.
- `masking_top`: the strongest overlaps between two stems in one band, score 0-1. Scores below
  0.05 are negligible. Overlaps between microphones of the same drum kit (kick, snare, toms,
  overheads, room) are physical bleed, not musical conflicts.
- One instrument may be split over several stems (e.g. a drum kit over 7 stems): their sum is
  louder than each stem alone, so balance groups, not just single stems.
- On close drum microphones, `active_pct` and `onsets_per_s` partly describe bleed from the rest
  of the kit; trust the declared role.
- The reverb return comes out about 1-7 dB above the send level (more with a larger
  `room_size`): choose sends and `return_gain_db` with that in mind.

## Verification loop
After each iteration the engine renders your plan, measures the mix and, from the second
iteration on, shows you the last iterations as `plan` + `evaluation` in the history:
- `spectral_distance_db`: mean absolute difference in dB between the 8 bands of the mix and of
  the target, after aligning their mean level. Lower is better. The loop stops when it falls
  below {DISTANCE_THRESHOLD_DB:g} dB, when it improves by less than {MIN_IMPROVEMENT_DB:g} dB on the
  previous iteration, or at the maximum number of iterations.
- `band_delta_db`: target minus mix per band, after that alignment: positive means the mix lacks
  energy in that band, negative means it has too much.
- `width_delta_db` (side/mid ratio) and `dyn_delta_lu` (loudness range): target minus mix;
  positive means the mix is narrower, or less dynamic, than the target.
- `masking_top`: the strongest collisions left between your processed stems.
- `constraints`: true peak within the ceiling, L/R correlation below 120 Hz above 0 (mono
  compatibility of the lows), negligible DC offset. A plan that breaks one is never chosen.
- `is_best`: whether that iteration was the best one so far; `mix`: measures of its mix.
Improve on the best iteration: keep what worked and change what the evaluation points at.
Without a target there is no distance: then only the constraints are checked.
{_revision_section(inp)}
## Output
Return one JSON object only: the complete mix plan with every stem of the project and every
field, never a partial plan or a diff. Use `"version": 1` and `"iteration": {inp.iteration}`.
For each stem write a `rationale` of one or two sentences explaining its settings, and a
`summary` of the whole plan. Write `rationale` and `summary` in the language of the brief.
"""


def _revision_section(inp: DecisionInput) -> str:
    """D-062: only for a revision, i.e. when the author made a request."""
    if not inp.request:
        return ""
    gain, eq = MAX_GAIN_CHANGE_DB, MAX_EQ_CHANGE_DB
    return f"""
## Revision
This is a revision asked by the author after listening to the mix of the starting plan (shown
with its evaluation in the user message). The author's request has top priority over the brief
and the target. Start from the starting plan and change what the request is about, including
the stems that cover or mask the one the request names (for example cut them where they collide
with it); keep the other parameters unless a change is needed to fulfil the request. The change
limits above apply from the starting plan (±{gain:g} dB of `gain_db`, ±{eq:g} dB per EQ band):
to go further, combine several parameters. There is only this iteration and no automatic
rollback: the author judges the result by ear. Here the spectral distance from the target is
only information, not the goal. In the rationale of every stem you change, say how the change
answers the request.
"""


def _brief(inp: DecisionInput) -> dict[str, str]:
    return {"genre": inp.brief.genre, "mood": inp.brief.mood, "notes": inp.brief.notes}


def _stems(inp: DecisionInput) -> list[dict[str, Any]]:
    out = []
    for s in inp.stems:
        entry: dict[str, Any] = {"id": s.id, "role": s.role}
        if s.notes:
            entry["notes"] = s.notes
        out.append(entry)
    return out


def _compact(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def user_message(inp: DecisionInput) -> str:
    parts = [
        "## Brief",
        json.dumps(_brief(inp), ensure_ascii=False, indent=2),
        "## Stems",
        "\n".join(_compact(s) for s in _stems(inp)),
        "## Analysis",
        inp.analysis_json.rstrip("\n"),
    ]
    history = inp.recent_history
    if history and inp.request:
        parts.append("## Starting plan (the mix the author listened to), with its evaluation")
    elif history:
        parts.append(f"## History (last {len(history)} iterations, oldest first)")
    for entry in history:
        parts.append(
            _compact({"plan": entry.plan.model_dump(mode="json"), "evaluation": entry.evaluation})
        )
    if inp.request:
        parts += [
            "## Author's request (top priority)",
            inp.request.strip(),
        ]
    task = (
        f"Revise the starting plan as the author asks and return the complete mix plan for "
        f"iteration {inp.iteration}."
        if inp.request
        else f"Decide the complete mix plan for iteration {inp.iteration}."
    )
    parts += ["## Task", task]
    return "\n\n".join(parts) + "\n"


def build_messages(inp: DecisionInput) -> list[dict[str, str]]:
    """The chat messages of a decision: system prompt and user message (DEC-1)."""
    return [
        {"role": "system", "content": system_prompt(inp)},
        {"role": "user", "content": user_message(inp)},
    ]
