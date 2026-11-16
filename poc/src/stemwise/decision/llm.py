"""Real decider: one chat completion on OpenRouter (or any compatible endpoint) per attempt.

- Before the call, the model's `supported_parameters` are read from `GET /models` (D-045):
  `temperature` and `seed` are sent only when supported; structured output (`json_schema`,
  strict, per-project schema, D-039) when the model supports it, otherwise JSON mode, otherwise
  the prompt alone; `reasoning.effort` when configured and supported. A parameter that is not
  sent gives a warning. `provider.require_parameters` routes only to providers honouring them.
- An invalid plan is sent back to the model with the errors, up to 2 more times (DEC-4, D-040);
  network and HTTP errors are not retried.
- Every attempt is logged in full (DEC-8): body sent (no headers, so no key), body received,
  status, duration, tokens, cost when the response reports it.
"""

from __future__ import annotations

import copy
import logging
import time
from typing import Any

import httpx

from stemwise.decision.base import Decision, DecisionInput
from stemwise.decision.config import LlmConfig
from stemwise.decision.prompt import build_messages
from stemwise.decision.schema import project_plan_schema
from stemwise.decision.validate import parse_response
from stemwise.errors import DecisionError, PlanError

log = logging.getLogger(__name__)

MAX_ATTEMPTS = 3  # the first one plus 2 retries (DEC-4)
SEED = 0
CONNECT_TIMEOUT_S = 15.0
SCHEMA_NAME = "mix_plan"
# Some providers reject an empty assistant message: an empty answer is echoed with this text.
EMPTY_ANSWER = "(empty answer)"


def _retry_message(errors: str) -> str:
    return (
        "Your answer is not a valid mix plan:\n"
        f"{errors}\n"
        "Answer again with the complete, corrected mix plan: one JSON object only, with every "
        "stem and every field."
    )


def _usage_totals(attempts: list[dict[str, Any]]) -> dict[str, Any]:
    def total(key: str) -> int:
        return sum(int((a.get("usage") or {}).get(key) or 0) for a in attempts)

    reasoning = sum(
        int(
            ((a.get("usage") or {}).get("completion_tokens_details") or {}).get("reasoning_tokens")
            or 0
        )
        for a in attempts
    )
    costs = [a.get("cost_usd") for a in attempts]
    known = [c for c in costs if c is not None]
    return {
        "attempts": len(attempts),
        "prompt_tokens": total("prompt_tokens"),
        "completion_tokens": total("completion_tokens"),
        "reasoning_tokens": reasoning,
        "cost_usd": round(sum(known), 6) if known else None,
        "cost_complete": len(known) == len(costs),
        "duration_s": sum(a.get("duration_s") or 0.0 for a in attempts),
    }


class LlmDecider:
    """Decider backed by a chat-completions endpoint (DEC-2, DEC-4, DEC-6, DEC-8)."""

    name = "llm"

    def __init__(self, config: LlmConfig, *, transport: httpx.BaseTransport | None = None) -> None:
        self.config = config
        self._transport = transport
        self._params: list[str] | None = None
        # Logs of the decision in progress, kept up to date call by call, so that a run can
        # still save them if the decision is interrupted by something else than DecisionError.
        self.last_request_log: dict[str, Any] | None = None
        self.last_response_log: dict[str, Any] | None = None

    def preflight(self) -> None:
        """Check, before any other work, that the model exists and read its parameters (D-044)."""
        with self._client() as client:
            self._params = self._model_parameters(client)

    # --- HTTP ---------------------------------------------------------------------------------

    def _client(self) -> httpx.Client:
        return httpx.Client(
            transport=self._transport,
            timeout=httpx.Timeout(self.config.timeout_s, connect=CONNECT_TIMEOUT_S),
            headers={"Authorization": f"Bearer {self.config.api_key}"},
        )

    def _http_error(self, exc: httpx.HTTPError, url: str) -> str:
        if isinstance(exc, httpx.TimeoutException):
            return f"the request to {url} timed out ({type(exc).__name__})"
        return f"cannot reach {url}: {type(exc).__name__}"

    def _model_parameters(self, client: httpx.Client) -> list[str]:
        url = f"{self.config.base_url}/models"
        try:
            response = client.get(url)
        except httpx.HTTPError as exc:
            raise DecisionError(self._http_error(exc, url)) from None
        if response.status_code != 200:
            raise DecisionError(f"HTTP {response.status_code} from {url}: cannot list the models")
        try:
            models = response.json()["data"]
        except (ValueError, KeyError, TypeError):
            raise DecisionError(f"unexpected answer from {url}: no model list") from None
        for m in models:
            if isinstance(m, dict) and m.get("id") == self.config.model:
                return sorted(str(p) for p in m.get("supported_parameters") or [])
        raise DecisionError(f"model {self.config.model} is not available at {self.config.base_url}")

    # --- request ------------------------------------------------------------------------------

    def _body(self, inp: DecisionInput, params: list[str], warnings: list[str]) -> dict[str, Any]:
        cfg = self.config
        body: dict[str, Any] = {"model": cfg.model, "messages": build_messages(inp)}
        if "structured_outputs" in params:
            body["response_format"] = {
                "type": "json_schema",
                "json_schema": {
                    "name": SCHEMA_NAME,
                    "strict": True,
                    "schema": project_plan_schema(inp.stem_ids),
                },
            }
        elif "response_format" in params:
            body["response_format"] = {"type": "json_object"}
            warnings.append(
                f"{cfg.model} does not support structured outputs: JSON mode plus validation"
            )
        else:
            warnings.append(
                f"{cfg.model} supports neither structured outputs nor JSON mode: "
                "the plan relies on the prompt and the validation"
            )
        optional = [("temperature", "temperature", cfg.temperature), ("seed", "seed", SEED)]
        if cfg.reasoning_effort is not None:
            optional.append(("reasoning", "reasoning", {"effort": cfg.reasoning_effort}))
        for name, key, value in optional:
            if name in params:
                body[key] = value
            else:
                warnings.append(f"{name} is not supported by {cfg.model}: not sent")
        body["provider"] = {"require_parameters": True}
        body["usage"] = {"include": True}
        return body

    # --- decision -----------------------------------------------------------------------------

    def decide(self, decision_input: DecisionInput) -> Decision:
        inp = decision_input
        cfg = self.config
        url = f"{cfg.base_url}/chat/completions"
        warnings: list[str] = []
        request_log: dict[str, Any] = {
            "decider": self.name,
            "config": cfg.public(),
            "url": url,
            "model_parameters": [],
            "warnings": warnings,
            "attempts": [],
        }
        response_log: dict[str, Any] = {"attempts": []}
        self.last_request_log, self.last_response_log = request_log, response_log

        def fail(message: str) -> DecisionError:
            response_log["total"] = _usage_totals(response_log["attempts"])
            return DecisionError(message, request_log=request_log, response_log=response_log)

        with self._client() as client:
            params = self._params if self._params is not None else self._model_parameters(client)
            request_log["model_parameters"] = params
            body = self._body(inp, params, warnings)
            for w in warnings:
                log.warning("%s", w)
            last_errors = ""
            for attempt in range(1, MAX_ATTEMPTS + 1):
                sent = copy.deepcopy(body)
                request_log["attempts"].append({"attempt": attempt, "body": sent})
                entry: dict[str, Any] = {"attempt": attempt}
                response_log["attempts"].append(entry)
                start = time.perf_counter()
                try:
                    response = client.post(url, json=sent)
                except httpx.HTTPError as exc:
                    entry.update(duration_s=time.perf_counter() - start, error=type(exc).__name__)
                    raise fail(self._http_error(exc, url)) from None
                entry.update(status=response.status_code, duration_s=time.perf_counter() - start)
                try:
                    payload = response.json()
                except ValueError:
                    entry["body"] = response.text
                    raise fail(
                        f"HTTP {response.status_code} from {url}: the answer is not JSON"
                    ) from None
                entry["body"] = payload
                if (
                    response.status_code != 200
                    or not isinstance(payload, dict)
                    or "error" in payload
                ):
                    error = payload.get("error") if isinstance(payload, dict) else None
                    detail = error.get("message") if isinstance(error, dict) else str(error)
                    code = error.get("code") if isinstance(error, dict) else None
                    raise fail(
                        f"HTTP {response.status_code} from {url}"
                        + (f" (code {code})" if code is not None else "")
                        + f": {detail}"
                    )
                usage = payload.get("usage") or {}
                cost = usage.get("cost")
                entry.update(
                    usage=usage, cost_usd=float(cost) if isinstance(cost, int | float) else None
                )
                try:
                    choice = payload["choices"][0]
                    message = choice.get("message") or {}
                    content = message.get("content")
                    finish_reason = choice.get("finish_reason")
                except (KeyError, IndexError, TypeError, AttributeError):
                    content, finish_reason = None, None
                entry["finish_reason"] = finish_reason
                try:
                    plan = parse_response(
                        content if isinstance(content, str) else None,
                        finish_reason,
                        inp.stem_ids,
                        inp.sample_rate,
                        iteration=inp.iteration,
                    )
                except PlanError as exc:
                    last_errors = str(exc)
                    entry.update(valid=False, errors=last_errors)
                    log.warning("attempt %d: invalid plan from the model", attempt)
                    body["messages"] = [
                        *body["messages"],
                        {
                            "role": "assistant",
                            "content": content
                            if isinstance(content, str) and content.strip()
                            else EMPTY_ANSWER,
                        },
                        {"role": "user", "content": _retry_message(last_errors)},
                    ]
                    continue
                entry["valid"] = True
                response_log["total"] = _usage_totals(response_log["attempts"])
                return Decision(
                    plan=plan,
                    warnings=tuple(warnings),
                    request_log=request_log,
                    response_log=response_log,
                )
        raise fail(
            f"the model returned no valid plan in {MAX_ATTEMPTS} attempts; last errors:\n"
            f"{last_errors}"
        )
