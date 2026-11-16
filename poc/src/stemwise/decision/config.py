"""Configuration of the real decider (DEC-6, D-037).

Values come, in order of precedence, from the command line, the environment and a `.env` file
(lines `KEY=value`). There is no default model: the model is a choice of the machine, never of
the code. The API key is kept out of `repr` so it cannot leak into logs or error messages.
"""

from __future__ import annotations

import math
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path

from stemwise.errors import DecisionError

ENV_API_KEY = "OPENROUTER_API_KEY"
ENV_MODEL = "STEMWISE_MODEL"
ENV_TEMPERATURE = "STEMWISE_TEMPERATURE"
ENV_BASE_URL = "STEMWISE_BASE_URL"
ENV_REASONING_EFFORT = "STEMWISE_REASONING_EFFORT"

DEFAULT_TEMPERATURE = 0.2
DEFAULT_BASE_URL = "https://openrouter.ai/api/v1"
DEFAULT_TIMEOUT_S = 300.0
REASONING_EFFORTS = ("low", "medium", "high", "xhigh")
MAX_TEMPERATURE = 2.0


@dataclass(frozen=True)
class LlmConfig:
    model: str
    api_key: str = field(repr=False)
    temperature: float = DEFAULT_TEMPERATURE
    base_url: str = DEFAULT_BASE_URL
    reasoning_effort: str | None = None
    timeout_s: float = DEFAULT_TIMEOUT_S

    def public(self) -> dict[str, object]:
        """The settings without the key, as written in the logs."""
        return {
            "model": self.model,
            "temperature": self.temperature,
            "base_url": self.base_url,
            "reasoning_effort": self.reasoning_effort,
            "timeout_s": self.timeout_s,
        }


def _unquote(value: str) -> str:
    if len(value) >= 2 and value[0] == value[-1] and value[0] in "'\"":
        return value[1:-1]
    return value


def read_env_file(path: Path) -> dict[str, str]:
    """Parse a minimal `.env`: `KEY=value` lines, `#` comments, optional `export ` and quotes."""
    values: dict[str, str] = {}
    for raw in Path(path).read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, _, value = line.partition("=")
        key = key.strip().removeprefix("export ").strip()
        if not key or " " in key:
            continue
        values[key] = _unquote(value.strip())
    return values


def _parse_temperature(value: str | float) -> float:
    try:
        t = float(value)
    except (TypeError, ValueError):
        raise DecisionError(f"{ENV_TEMPERATURE} must be a number, got {value!r}") from None
    if not (math.isfinite(t) and 0.0 <= t <= MAX_TEMPERATURE):
        raise DecisionError(f"{ENV_TEMPERATURE} must be between 0 and {MAX_TEMPERATURE:g}, got {t}")
    return t


def load_llm_config(
    environ: Mapping[str, str],
    *,
    env_file: Path | None = Path(".env"),
    model: str | None = None,
    temperature: float | None = None,
    reasoning_effort: str | None = None,
) -> LlmConfig:
    """Resolve the configuration: arguments > `environ` > `env_file` > defaults."""
    values: dict[str, str] = {}
    if env_file is not None and Path(env_file).is_file():
        values.update({k: v for k, v in read_env_file(Path(env_file)).items() if v != ""})
    values.update({k: v for k, v in environ.items() if v != ""})

    api_key = values.get(ENV_API_KEY, "").strip()
    if not api_key:
        raise DecisionError(f"{ENV_API_KEY} is not set (environment or .env)")
    chosen_model = (model or values.get(ENV_MODEL, "")).strip()
    if not chosen_model:
        raise DecisionError(
            f"no model configured: set {ENV_MODEL} (environment or .env) or --model"
        )

    temp = _parse_temperature(
        temperature if temperature is not None else values.get(ENV_TEMPERATURE, DEFAULT_TEMPERATURE)
    )
    effort = reasoning_effort or values.get(ENV_REASONING_EFFORT) or None
    if effort is not None and effort not in REASONING_EFFORTS:
        raise DecisionError(
            f"{ENV_REASONING_EFFORT} must be one of {', '.join(REASONING_EFFORTS)}, got {effort!r}"
        )
    base_url = values.get(ENV_BASE_URL, DEFAULT_BASE_URL).strip().rstrip("/")
    if not base_url.startswith(("https://", "http://")):
        raise DecisionError(f"{ENV_BASE_URL} must be an http(s) URL, got {base_url!r}")
    return LlmConfig(
        model=chosen_model,
        api_key=api_key,
        temperature=temp,
        base_url=base_url,
        reasoning_effort=effort,
    )
