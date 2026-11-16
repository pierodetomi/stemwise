"""Real decider against a fake OpenRouter (DEC-2, DEC-4, DEC-6, DEC-8, D-039, D-040, D-045)."""

from __future__ import annotations

import json
from typing import Any

import httpx
import pytest

from stemwise.decision.base import Decider
from stemwise.decision.config import LlmConfig
from stemwise.decision.llm import EMPTY_ANSWER, MAX_ATTEMPTS, LlmDecider
from stemwise.decision.prompt import build_messages
from stemwise.decision.schema import project_plan_schema
from stemwise.errors import DecisionError
from tests.helpers.decision import make_input
from tests.helpers.openrouter import (
    ALL_PARAMETERS,
    KEY,
    MODEL,
    FakeOpenRouter,
    completion,
    valid_plan_text,
)
from tests.helpers.plan import example_plan

IDS = ["kick", "bass", "gtr", "lead_vox"]


def _config(**overrides: Any) -> LlmConfig:
    return LlmConfig(**{"model": MODEL, "api_key": KEY, **overrides})


def _decider(fake: FakeOpenRouter, **overrides: Any) -> LlmDecider:
    return LlmDecider(_config(**overrides), transport=fake.transport())


def _invalid_text() -> str:
    doc = example_plan(IDS)
    doc["stems"]["gtr"]["gain_db"] = 30.0
    return json.dumps(doc)


def test_is_a_decider() -> None:
    assert isinstance(_decider(FakeOpenRouter([])), Decider)


@pytest.mark.req("DEC-2", "DEC-1")
def test_request_uses_project_schema() -> None:
    fake = FakeOpenRouter([valid_plan_text(IDS)])
    inp = make_input()
    decision = _decider(fake).decide(inp)
    assert set(decision.plan.stems) == set(IDS)
    (body,) = fake.chat_bodies()
    assert body["model"] == MODEL
    assert body["messages"] == build_messages(inp)
    assert body["response_format"] == {
        "type": "json_schema",
        "json_schema": {"name": "mix_plan", "strict": True, "schema": project_plan_schema(IDS)},
    }
    assert body["provider"] == {"require_parameters": True}
    assert body["usage"] == {"include": True}


@pytest.mark.req("DEC-2")
def test_json_object_mode() -> None:
    params = tuple(p for p in ALL_PARAMETERS if p != "structured_outputs")
    fake = FakeOpenRouter([valid_plan_text(IDS)], parameters=params)
    decision = _decider(fake).decide(make_input())
    assert fake.chat_bodies()[0]["response_format"] == {"type": "json_object"}
    assert any("structured outputs" in w for w in decision.warnings)


@pytest.mark.req("DEC-2")
def test_no_json_mode_at_all_relies_on_validation() -> None:
    params = ("temperature",)
    fake = FakeOpenRouter([valid_plan_text(IDS)], parameters=params)
    decision = _decider(fake).decide(make_input())
    assert "response_format" not in fake.chat_bodies()[0]
    assert any("JSON mode" in w for w in decision.warnings)


@pytest.mark.req("DEC-6")
@pytest.mark.parametrize(
    ("params", "effort", "expected", "warned"),
    [
        (ALL_PARAMETERS, None, {"temperature": 0.2, "seed": 0}, []),
        (ALL_PARAMETERS, "high", {"temperature": 0.2, "seed": 0, "reasoning": "high"}, []),
        (
            ("structured_outputs", "reasoning"),
            "xhigh",
            {"reasoning": "xhigh"},
            ["temperature", "seed"],
        ),
        (("structured_outputs",), "low", {}, ["temperature", "seed", "reasoning"]),
    ],
)
def test_parameters_follow_model_support(
    params: tuple[str, ...], effort: str | None, expected: dict[str, Any], warned: list[str]
) -> None:
    fake = FakeOpenRouter([valid_plan_text(IDS)], parameters=params)
    decision = _decider(fake, reasoning_effort=effort).decide(make_input())
    body = fake.chat_bodies()[0]
    sent = {k: body[k] for k in ("temperature", "seed") if k in body}
    if "reasoning" in body:
        sent["reasoning"] = body["reasoning"]["effort"]
    assert sent == expected
    for name in warned:
        assert any(f"{name} is not supported by {MODEL}" in w for w in decision.warnings), name
    assert decision.request_log["model_parameters"] == sorted(params)


@pytest.mark.req("DEC-6")
def test_temperature_and_base_url_are_configurable() -> None:
    fake = FakeOpenRouter([valid_plan_text(IDS)])
    _decider(fake, temperature=0.7, base_url="https://llm.example.test/v9").decide(make_input())
    assert fake.chat_bodies()[0]["temperature"] == 0.7
    assert {str(r.url) for r in fake.requests} == {
        "https://llm.example.test/v9/models",
        "https://llm.example.test/v9/chat/completions",
    }
    auth = fake.chat_requests[0].headers["authorization"]
    assert auth == f"Bearer {KEY}"


def test_unknown_model_fails_before_any_call() -> None:
    fake = FakeOpenRouter([valid_plan_text(IDS)], model="vendor/another")
    with pytest.raises(DecisionError, match=f"model {MODEL} is not available"):
        _decider(fake).decide(make_input())
    assert fake.chat_requests == []


@pytest.mark.req("DEC-4")
def test_retry_sends_errors_back() -> None:
    fake = FakeOpenRouter([_invalid_text(), valid_plan_text(IDS)])
    decision = _decider(fake).decide(make_input())
    first, second = fake.chat_bodies()
    assert len(second["messages"]) == len(first["messages"]) + 2
    assert second["messages"][:-2] == first["messages"]
    assert second["messages"][-2] == {"role": "assistant", "content": _invalid_text()}
    feedback = second["messages"][-1]
    assert feedback["role"] == "user"
    assert "stems.gtr.gain_db" in feedback["content"]
    assert "complete" in feedback["content"]
    assert decision.plan.stems["gtr"].gain_db == 2.0
    attempts = decision.response_log["attempts"]
    assert [a["valid"] for a in attempts] == [False, True]
    assert "stems.gtr.gain_db" in attempts[0]["errors"]


@pytest.mark.req("DEC-4")
def test_gives_up_after_two_retries() -> None:
    fake = FakeOpenRouter([_invalid_text(), "not json", _invalid_text()])
    with pytest.raises(DecisionError, match="no valid plan in 3 attempts") as info:
        _decider(fake).decide(make_input())
    assert MAX_ATTEMPTS == 3
    assert len(fake.chat_requests) == 3  # never a fourth call
    assert "stems.gtr.gain_db" in str(info.value)  # errors of the last attempt
    assert len(info.value.request_log["attempts"]) == 3
    assert len(info.value.response_log["attempts"]) == 3


@pytest.mark.req("DEC-4")
@pytest.mark.parametrize(
    "first",
    [
        "Sure, here is a great mix for your song!",
        "",
        None,
        completion(valid_plan_text(IDS)[:200], finish_reason="length"),
    ],
)
def test_recovers_from_non_json_answer(first: Any) -> None:
    fake = FakeOpenRouter([first, valid_plan_text(IDS)])
    decision = _decider(fake).decide(make_input())
    assert len(fake.chat_requests) == 2
    assert set(decision.plan.stems) == set(IDS)


def _status(code: int) -> httpx.Response:
    return httpx.Response(code, json={"error": {"message": f"upstream said {code}", "code": code}})


def _raise(exc: type[httpx.HTTPError]):
    def handler(request: httpx.Request) -> httpx.Response:
        raise exc("boom", request=request)

    return handler


@pytest.mark.req("DEC-4")
@pytest.mark.parametrize(
    ("answer", "message"),
    [
        (_status(401), "HTTP 401"),
        (_status(429), "HTTP 429"),
        (_status(500), "HTTP 500"),
        (httpx.Response(200, json={"error": {"message": "provider failed", "code": 502}}), "502"),
        (httpx.Response(200, text="<html>gateway</html>"), "not JSON"),
        (_raise(httpx.ReadTimeout), "timed out"),
        (_raise(httpx.ConnectError), "cannot reach"),
    ],
)
def test_http_errors_are_not_retried(answer: Any, message: str) -> None:
    fake = FakeOpenRouter([answer, valid_plan_text(IDS)])
    with pytest.raises(DecisionError, match=message) as info:
        _decider(fake).decide(make_input())
    assert len(fake.chat_requests) == 1
    assert KEY not in str(info.value)
    assert len(info.value.request_log["attempts"]) == 1
    assert len(info.value.response_log["attempts"]) == 1


@pytest.mark.req("DEC-8")
def test_logs_are_complete() -> None:
    no_cost = {"prompt_tokens": 1500, "completion_tokens": 300, "total_tokens": 1800}
    fake = FakeOpenRouter([_invalid_text(), completion(valid_plan_text(IDS), usage=no_cost)])
    decision = _decider(fake, reasoning_effort="high").decide(make_input())
    req, resp = decision.request_log, decision.response_log
    assert req["decider"] == "llm"
    assert req["config"] == {
        "model": MODEL,
        "temperature": 0.2,
        "base_url": "https://openrouter.ai/api/v1",
        "reasoning_effort": "high",
        "timeout_s": 300.0,
    }
    assert req["url"] == "https://openrouter.ai/api/v1/chat/completions"
    assert [a["body"] for a in req["attempts"]] == fake.chat_bodies()  # sent in full
    first, second = resp["attempts"]
    assert [first["attempt"], second["attempt"]] == [1, 2]
    assert first["status"] == 200
    assert first["duration_s"] >= 0.0
    assert first["body"]["choices"][0]["message"]["content"] == _invalid_text()  # received in full
    assert first["usage"]["prompt_tokens"] == 1000
    assert first["cost_usd"] == 0.0021
    assert second["cost_usd"] is None  # not reported by the response
    assert resp["total"] == {
        "attempts": 2,
        "prompt_tokens": 2500,
        "completion_tokens": 500,
        "reasoning_tokens": 50,
        "cost_usd": 0.0021,
        "cost_complete": False,
        "duration_s": pytest.approx(first["duration_s"] + second["duration_s"]),
    }


@pytest.mark.req("DEC-8")
def test_api_key_never_in_logs_or_warnings() -> None:
    fake = FakeOpenRouter([_invalid_text(), valid_plan_text(IDS)], parameters=("reasoning",))
    decision = _decider(fake, reasoning_effort="low").decide(make_input())
    text = json.dumps([decision.request_log, decision.response_log, decision.warnings])
    assert KEY not in text


@pytest.mark.req("DEC-4")
@pytest.mark.parametrize("empty", ["", None, "  "])
def test_empty_answer_is_echoed_with_text(empty: str | None) -> None:
    """Some providers reject an empty assistant message: the retry must still be accepted."""
    fake = FakeOpenRouter([empty, valid_plan_text(IDS)])
    _decider(fake).decide(make_input())
    echoed = fake.chat_bodies()[1]["messages"][-2]
    assert echoed == {"role": "assistant", "content": EMPTY_ANSWER}
    assert all(m["content"].strip() for m in fake.chat_bodies()[1]["messages"])


def test_preflight_reads_parameters_once() -> None:
    fake = FakeOpenRouter([valid_plan_text(IDS)])
    decider = _decider(fake)
    decider.preflight()
    decider.decide(make_input())
    assert [r.url.path for r in fake.requests] == ["/api/v1/models", "/api/v1/chat/completions"]
