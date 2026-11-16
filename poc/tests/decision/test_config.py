"""Configuration of the real decider (DEC-6, D-037) and the `Decider` interface (PRD §7)."""

from __future__ import annotations

import re
from pathlib import Path

import pytest

from stemwise.decision.base import Decider
from stemwise.decision.config import (
    DEFAULT_BASE_URL,
    DEFAULT_TEMPERATURE,
    load_llm_config,
    read_env_file,
)
from stemwise.errors import DecisionError

SRC = Path(__file__).resolve().parents[2] / "src"
KEY = "sk-test-not-a-real-key"


def _env(**values: str) -> dict[str, str]:
    return {"OPENROUTER_API_KEY": KEY, "STEMWISE_MODEL": "vendor/model-a", **values}


@pytest.mark.req("DEC-6")
def test_defaults() -> None:
    cfg = load_llm_config(_env(), env_file=None)
    assert cfg.model == "vendor/model-a"
    assert cfg.temperature == DEFAULT_TEMPERATURE == 0.2
    assert cfg.base_url == DEFAULT_BASE_URL == "https://openrouter.ai/api/v1"
    assert cfg.reasoning_effort is None
    assert cfg.api_key == KEY


@pytest.mark.req("DEC-6")
def test_environment_values() -> None:
    cfg = load_llm_config(
        _env(
            STEMWISE_TEMPERATURE="0.7",
            STEMWISE_BASE_URL="https://example.test/api/v1/",
            STEMWISE_REASONING_EFFORT="high",
        ),
        env_file=None,
    )
    assert cfg.temperature == 0.7
    assert cfg.base_url == "https://example.test/api/v1"  # trailing slash removed
    assert cfg.reasoning_effort == "high"


@pytest.mark.req("DEC-6")
def test_precedence_cli_over_environment_over_env_file(tmp_path: Path) -> None:
    env_file = tmp_path / ".env"
    env_file.write_text(
        "# comment\n"
        f"OPENROUTER_API_KEY={KEY}\n"
        "export STEMWISE_MODEL='vendor/from-file'\n"
        'STEMWISE_TEMPERATURE="0.5"\n'
        "STEMWISE_REASONING_EFFORT=low\n"
        "\n",
        encoding="utf-8",
    )
    from_file = load_llm_config({}, env_file=env_file)
    assert (from_file.model, from_file.temperature, from_file.reasoning_effort) == (
        "vendor/from-file",
        0.5,
        "low",
    )
    from_env = load_llm_config({"STEMWISE_MODEL": "vendor/from-env"}, env_file=env_file)
    assert from_env.model == "vendor/from-env"
    assert from_env.temperature == 0.5  # not in the environment: still from the file
    from_cli = load_llm_config(
        {"STEMWISE_MODEL": "vendor/from-env"},
        env_file=env_file,
        model="vendor/from-cli",
        temperature=0.0,
        reasoning_effort="xhigh",
    )
    assert (from_cli.model, from_cli.temperature, from_cli.reasoning_effort) == (
        "vendor/from-cli",
        0.0,
        "xhigh",
    )


def test_missing_env_file_is_fine(tmp_path: Path) -> None:
    assert load_llm_config(_env(), env_file=tmp_path / "absent.env").model == "vendor/model-a"


def test_read_env_file_ignores_malformed_lines(tmp_path: Path) -> None:
    path = tmp_path / ".env"
    path.write_text("NOT A PAIR\n=value\nA=1\nB = two words \n", encoding="utf-8")
    assert read_env_file(path) == {"A": "1", "B": "two words"}


@pytest.mark.req("DEC-6")
@pytest.mark.parametrize(
    ("env", "message"),
    [
        ({"STEMWISE_MODEL": "vendor/m"}, "OPENROUTER_API_KEY"),
        ({"OPENROUTER_API_KEY": KEY}, "STEMWISE_MODEL"),
        ({**_env(), "STEMWISE_TEMPERATURE": "warm"}, "STEMWISE_TEMPERATURE"),
        ({**_env(), "STEMWISE_TEMPERATURE": "2.5"}, "STEMWISE_TEMPERATURE"),
        ({**_env(), "STEMWISE_REASONING_EFFORT": "huge"}, "STEMWISE_REASONING_EFFORT"),
        ({**_env(), "STEMWISE_BASE_URL": "ftp://x"}, "STEMWISE_BASE_URL"),
    ],
)
def test_invalid_configuration_fails_clearly(env: dict[str, str], message: str) -> None:
    with pytest.raises(DecisionError, match=message) as info:
        load_llm_config(env, env_file=None)
    assert KEY not in str(info.value)


def test_api_key_is_not_in_repr() -> None:
    cfg = load_llm_config(_env(), env_file=None)
    assert KEY not in repr(cfg)
    assert KEY not in str(cfg)


@pytest.mark.req("DEC-6")
def test_no_model_hardcoded() -> None:
    """DEC-6: no model id (`<provider>/<model>`) anywhere in the source code."""
    providers = "openai|anthropic|google|x-ai|deepseek|qwen|moonshotai|z-ai|mistralai|meta-llama"
    pattern = re.compile(rf"\b(?:{providers})/[a-z0-9]")
    hits = [
        f"{path.relative_to(SRC)}:{n}"
        for path in SRC.rglob("*.py")
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1)
        if pattern.search(line)
    ]
    assert hits == []


def test_decider_is_a_protocol() -> None:
    class Fake:
        name = "fake"

        def decide(self, decision_input):  # pragma: no cover - never called
            raise NotImplementedError

    assert isinstance(Fake(), Decider)


@pytest.mark.req("DEC-6")
def test_empty_values_mean_default(tmp_path: Path) -> None:
    """A copied .env.example leaves optional values empty: they fall back to the defaults."""
    env_file = tmp_path / ".env"
    env_file.write_text(
        f"OPENROUTER_API_KEY={KEY}\nSTEMWISE_MODEL=vendor/m\nSTEMWISE_REASONING_EFFORT=\n"
        "STEMWISE_TEMPERATURE=\nSTEMWISE_BASE_URL=\n",
        encoding="utf-8",
    )
    cfg = load_llm_config({"STEMWISE_TEMPERATURE": ""}, env_file=env_file)
    assert (cfg.reasoning_effort, cfg.temperature, cfg.base_url) == (
        None,
        DEFAULT_TEMPERATURE,
        DEFAULT_BASE_URL,
    )
