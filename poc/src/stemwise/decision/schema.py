"""Contract of `mix_plan.json` (PRD §6.3, D-027): the mix plan, with the limits of the parameters.

`MixPlan` is exported to `schemas/mix_plan.schema.json`. A plan is always complete (DEC-2): every
field is required, and `null` (or an empty `eq`) switches a block off. `check_plan_fits` adds the
checks that depend on the project: the stem ids (D-033) and the Nyquist frequency.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, ConfigDict, Field, ValidationError

from stemwise.errors import PlanError
from stemwise.project import StemId

PLAN_VERSION = 1
MAX_EQ_BANDS = 4
MAX_STEMS = 16


class _Contract(BaseModel):
    # strict: no coercion ("2" -> 2.0, true -> 1.0), so the engine accepts exactly the plans
    # that the exported JSON Schema accepts (PRD §7.1).
    model_config = ConfigDict(extra="forbid", allow_inf_nan=False, strict=True)


class EqBand(_Contract):
    type: Literal["peak", "low_shelf", "high_shelf"] = Field(description="RBJ biquad type.")
    freq_hz: float = Field(ge=20.0, le=20_000.0, description="Centre or corner frequency in Hz.")
    gain_db: float = Field(ge=-9.0, le=9.0, description="Gain in dB.")
    q: float = Field(ge=0.3, le=8.0, description="Quality factor (RBJ cookbook, Q form).")


class Compressor(_Contract):
    threshold_db: float = Field(ge=-40.0, le=0.0, description="Threshold in dBFS.")
    ratio: float = Field(ge=1.0, le=8.0, description="Compression ratio.")
    attack_ms: float = Field(ge=0.1, le=100.0, description="Attack time in ms.")
    release_ms: float = Field(ge=20.0, le=1_000.0, description="Release time in ms.")
    makeup_db: float = Field(ge=0.0, le=12.0, description="Makeup gain after compression, dB.")


class Reverb(_Contract):
    room_size: float = Field(ge=0.0, le=1.0, description="Room size, 0-1.")
    damping: float = Field(ge=0.0, le=1.0, description="High-frequency damping, 0-1.")
    return_hpf_hz: float = Field(ge=20.0, le=800.0, description="High-pass on the return, Hz.")
    return_gain_db: float = Field(ge=-24.0, le=0.0, description="Gain of the return, dB.")


class StemPlan(_Contract):
    gain_db: float = Field(ge=-24.0, le=12.0, description="Fader gain in dB.")
    pan: float = Field(ge=-1.0, le=1.0, description="Pan: -1 left, 0 centre, +1 right.")
    hpf_hz: float | None = Field(
        ge=20.0, le=400.0, description="High-pass cutoff in Hz (12 dB/octave), null = off."
    )
    lpf_hz: float | None = Field(
        ge=2_000.0, le=20_000.0, description="Low-pass cutoff in Hz (12 dB/octave), null = off."
    )
    eq: list[EqBand] | None = Field(
        max_length=MAX_EQ_BANDS, description="Up to 4 EQ bands, applied in order; null or [] = off."
    )
    compressor: Compressor | None = Field(description="Compressor, null = off.")
    reverb_send_db: float | None = Field(
        ge=-60.0, le=-6.0, description="Send level to the shared reverb in dB, null = no send."
    )
    rationale: str = Field(min_length=1, description="One or two sentences explaining the choices.")

    @property
    def eq_bands(self) -> list[EqBand]:
        return self.eq or []


class MixPlan(_Contract):
    """Content of mix_plan.json: a complete set of static mix parameters."""

    version: Literal[1] = Field(description="Version of the plan format.")
    iteration: int = Field(ge=0, description="Iteration of the verification loop.")
    summary: str = Field(min_length=1, description="Summary of the plan.")
    reverb: Reverb | None = Field(description="Shared reverb, null = off.")
    stems: dict[StemId, StemPlan] = Field(
        min_length=1, max_length=MAX_STEMS, description="Parameters for every stem, by stem id."
    )


def _format_validation_error(source: str, error: ValidationError) -> str:
    lines = [f"invalid mix plan ({source}):"]
    for item in error.errors():
        loc = ".".join(str(part) for part in item["loc"]) or "<root>"
        lines.append(f"  - {loc}: {item['msg']}")
    return "\n".join(lines)


def parse_plan(text: str, source: str = "<plan>") -> MixPlan:
    """Parse and validate a plan from JSON text. Raises `PlanError` with every violation."""
    try:
        doc = json.loads(text)
    except json.JSONDecodeError as exc:
        raise PlanError(f"mix plan ({source}) is not valid JSON: {exc}") from exc
    if not isinstance(doc, dict):
        raise PlanError(f"invalid mix plan ({source}): expected a JSON object at the top level")
    try:
        return MixPlan.model_validate(doc)
    except ValidationError as exc:
        raise PlanError(_format_validation_error(source, exc)) from exc


def load_plan(path: Path) -> MixPlan:
    path = Path(path)
    if not path.is_file():
        raise PlanError(f"mix plan not found: {path}")
    return parse_plan(path.read_text(encoding="utf-8"), str(path))


def dumps_plan(plan: MixPlan) -> str:
    return json.dumps(plan.model_dump(mode="json"), indent=2, ensure_ascii=False) + "\n"


def check_plan_fits(plan: MixPlan, stem_ids: list[str], sample_rate: int) -> None:
    """Checks that depend on the project (D-033): exactly its stems, frequencies below fs/2."""
    problems: list[str] = []
    missing = [sid for sid in stem_ids if sid not in plan.stems]
    unknown = [sid for sid in plan.stems if sid not in stem_ids]
    if missing:
        problems.append(f"stems missing from the plan: {', '.join(missing)}")
    if unknown:
        problems.append(f"stems not in the project: {', '.join(unknown)}")
    nyquist = sample_rate / 2.0
    freqs: list[tuple[str, float]] = []
    if plan.reverb is not None:
        freqs.append(("reverb.return_hpf_hz", plan.reverb.return_hpf_hz))
    for sid, sp in plan.stems.items():
        if sp.hpf_hz is not None:
            freqs.append((f"stems.{sid}.hpf_hz", sp.hpf_hz))
        if sp.lpf_hz is not None:
            freqs.append((f"stems.{sid}.lpf_hz", sp.lpf_hz))
        freqs += [(f"stems.{sid}.eq.{i}.freq_hz", b.freq_hz) for i, b in enumerate(sp.eq_bands)]
    problems += [
        f"{loc} = {f:g} Hz is not below the Nyquist frequency ({nyquist:g} Hz)"
        for loc, f in freqs
        if f >= nyquist
    ]
    if problems:
        raise PlanError(
            "mix plan does not fit the project:\n" + "\n".join(f"  - {p}" for p in problems)
        )


# Keywords outside the subset accepted by the `strict` structured-output modes of
# OpenAI-compatible providers. They are dropped from the schema *sent to the model* only; the
# local validation (`MixPlan`, `check_plan_fits`) still enforces them (D-039).
UNSUPPORTED_STRICT_KEYWORDS = frozenset(
    {
        "title",
        "default",
        "examples",
        "minLength",
        "maxLength",
        "minProperties",
        "maxProperties",
        "patternProperties",
        "propertyNames",
    }
)


def _strip_keywords(node: object) -> object:
    """Drop unsupported keywords; the keys of `properties` and `$defs` are names, kept as is."""
    if isinstance(node, list):
        return [_strip_keywords(v) for v in node]
    if not isinstance(node, dict):
        return node
    out: dict[str, object] = {}
    for key, value in node.items():
        if key in ("properties", "$defs") and isinstance(value, dict):
            out[key] = {name: _strip_keywords(sub) for name, sub in value.items()}
        elif key not in UNSUPPORTED_STRICT_KEYWORDS:
            out[key] = _strip_keywords(value)
    return out


def project_plan_schema(stem_ids: list[str]) -> dict:
    """JSON Schema of the plan for one project, for structured output (DEC-2, D-039).

    Derived from `MixPlan`: `stems` becomes an object whose properties are exactly the project's
    stem ids, all required, with no other key allowed; keywords outside the strict subset are
    removed. Names under `properties` are kept even when they look like keywords (a stem may be
    called `default` or `title`).
    """
    schema = MixPlan.model_json_schema()
    stems = schema["properties"]["stems"]
    schema["properties"]["stems"] = {
        "type": "object",
        "description": stems.get("description", ""),
        "properties": {sid: {"$ref": "#/$defs/StemPlan"} for sid in stem_ids},
        "required": list(stem_ids),
        "additionalProperties": False,
    }
    return _strip_keywords(schema)  # type: ignore[return-value]
