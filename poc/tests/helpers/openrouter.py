"""A fake OpenRouter for tests: `httpx.MockTransport` with scripted answers, no network."""

from __future__ import annotations

import json
from collections.abc import Callable
from dataclasses import dataclass, field
from typing import Any

import httpx

from tests.helpers.plan import example_plan

MODEL = "vendor/test-model"
KEY = "sk-or-test-0123456789-secret"
ALL_PARAMETERS = (
    "max_tokens",
    "reasoning",
    "response_format",
    "seed",
    "structured_outputs",
    "temperature",
)

Answer = str | None | Callable[[httpx.Request], httpx.Response] | httpx.Response


def completion(
    content: str | None,
    *,
    finish_reason: str = "stop",
    usage: dict[str, Any] | None = None,
) -> httpx.Response:
    body = {
        "id": "gen-test",
        "model": MODEL,
        "choices": [
            {
                "index": 0,
                "finish_reason": finish_reason,
                "message": {"role": "assistant", "content": content},
            }
        ],
        "usage": usage
        if usage is not None
        else {
            "prompt_tokens": 1000,
            "completion_tokens": 200,
            "total_tokens": 1200,
            "cost": 0.0021,
            "completion_tokens_details": {"reasoning_tokens": 50},
        },
    }
    return httpx.Response(200, json=body)


def valid_plan_text(stem_ids: list[str]) -> str:
    return json.dumps(example_plan(stem_ids))


@dataclass
class FakeOpenRouter:
    """Serves `/models` and `/chat/completions`; records every request it receives."""

    answers: list[Answer]
    parameters: tuple[str, ...] = ALL_PARAMETERS
    model: str = MODEL
    requests: list[httpx.Request] = field(default_factory=list)

    @property
    def chat_requests(self) -> list[httpx.Request]:
        return [r for r in self.requests if r.url.path.endswith("/chat/completions")]

    def chat_bodies(self) -> list[dict[str, Any]]:
        return [json.loads(r.content) for r in self.chat_requests]

    def handler(self, request: httpx.Request) -> httpx.Response:
        self.requests.append(request)
        if request.method == "GET" and request.url.path.endswith("/models"):
            data = [
                {"id": "other/model", "supported_parameters": ["temperature"]},
                {"id": self.model, "supported_parameters": list(self.parameters)},
            ]
            return httpx.Response(200, json={"data": data})
        if request.method == "POST" and request.url.path.endswith("/chat/completions"):
            if not self.answers:
                raise AssertionError("unexpected extra call to the model")
            answer = self.answers.pop(0)
            if isinstance(answer, httpx.Response):
                return answer
            if callable(answer):
                return answer(request)
            return completion(answer)
        return httpx.Response(404, json={"error": {"message": "not found"}})

    def transport(self) -> httpx.MockTransport:
        return httpx.MockTransport(self.handler)
