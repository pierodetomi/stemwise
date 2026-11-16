"""The real model (M3–M5 acceptance): plans, logs, the loop, a revision. Network, costs money.

Runs only with `make test-llm`; needs OPENROUTER_API_KEY and STEMWISE_MODEL (environment or
.env in the current folder), optionally STEMWISE_REASONING_EFFORT.
"""

from __future__ import annotations

import json
import os
from pathlib import Path

import pytest

from stemwise.decision.config import load_llm_config
from stemwise.decision.llm import LlmDecider
from stemwise.decision.schema import load_plan
from stemwise.errors import DecisionError
from stemwise.runs import mix_run
from tests.helpers.project import make_project

pytestmark = [pytest.mark.llm, pytest.mark.acceptance("M3"), pytest.mark.req("DEC-2", "DEC-8")]


def test_real_model_returns_valid_plan(tmp_path: Path) -> None:
    try:
        config = load_llm_config(os.environ, env_file=Path(".env"))
    except DecisionError as exc:
        pytest.fail(f"real-model test needs its configuration: {exc}")
    root = make_project(tmp_path / "song")
    run = mix_run(root, LlmDecider(config), max_iterations=1)
    plan = load_plan(run.iterations[0].plan_file)  # valid against the contract
    assert set(plan.stems) == {"kick", "bass", "gtr", "lead_vox"}  # key order is free
    assert all(sp.rationale.strip() for sp in plan.stems.values())
    it = run.run_dir / "iter_00"
    request = json.loads((it / "llm_request.json").read_text(encoding="utf-8"))
    response = json.loads((it / "llm_response.json").read_text(encoding="utf-8"))
    assert 1 <= len(request["attempts"]) == len(response["attempts"]) <= 3
    assert response["attempts"][-1]["valid"] is True
    total = response["total"]
    assert total["prompt_tokens"] > 0
    assert total["completion_tokens"] > 0
    assert total["duration_s"] > 0
    assert (it / "mix.wav").is_file()
    print(f"\nrun: {run.run_dir}\ncost: {total['cost_usd']} USD, tokens: {total}")


@pytest.mark.acceptance("M4")
@pytest.mark.req("EVA-5", "EVA-6", "REP-1")
def test_real_model_runs_the_loop(tmp_path: Path) -> None:
    """Two iterations on a synthetic project with a reference: history, evaluation, report."""
    from tests.helpers.signals import pink_noise

    try:
        config = load_llm_config(os.environ, env_file=Path(".env"))
    except DecisionError as exc:
        pytest.fail(f"real-model test needs its configuration: {exc}")
    reference = pink_noise(duration_s=4.0, channels=2, rms=0.1, seed=9)
    root = make_project(tmp_path / "song", references=[("ref.wav", reference, 48_000)])
    run = mix_run(root, LlmDecider(config), max_iterations=2)
    assert run.iterations, "no iteration was evaluated"
    assert run.report_file.is_file()
    best = json.loads(run.best_file.read_text(encoding="utf-8"))
    assert best["stop_reason"] is not None
    if len(run.iterations) == 2:
        request = json.loads((run.iterations[1].dir / "llm_request.json").read_text("utf-8"))
        user = request["attempts"][0]["body"]["messages"][1]["content"]
        assert "## History" in user
    costs = [
        json.loads((it.dir / "llm_response.json").read_text(encoding="utf-8"))["total"]["cost_usd"]
        for it in run.iterations
    ]
    print(f"\nrun: {run.run_dir}\nbest: {best}\ncost per iteration: {costs} USD")


@pytest.mark.acceptance("M5")
@pytest.mark.req("REV-1", "REV-3")
def test_real_model_revises_the_voice(tmp_path: Path) -> None:
    """One real revision from a free mock mix: the diff goes in the requested direction (H4)."""
    from stemwise.decision.mock import MockDecider
    from stemwise.revise import REVERB_TARGET, plan_diff
    from stemwise.runs import revise_run

    try:
        config = load_llm_config(os.environ, env_file=Path(".env"))
    except DecisionError as exc:
        pytest.fail(f"real-model test needs its configuration: {exc}")
    root = make_project(tmp_path / "song")
    mix_run(root, MockDecider(), max_iterations=1)  # free starting point
    run = revise_run(root, LlmDecider(config), "lead vocal more upfront")
    start = load_plan(run.run_dir / "source_plan.json")
    revised = load_plan(run.iterations[0].plan_file)  # valid against the contract
    changes = plan_diff(start, revised)
    louder_vocal = any(
        c.target == "lead_vox" and c.parameter == "gain_db" and c.after > c.before for c in changes
    )
    others_down = any(
        c.target not in ("lead_vox", REVERB_TARGET)
        and (
            (c.parameter == "gain_db" and c.after < c.before)
            or (c.parameter.endswith(".gain_db") and c.after < c.before)
            or (c.parameter.startswith("eq[") and c.before is None and " -" in str(c.after))
        )
        for c in changes
    )
    assert louder_vocal or others_down, changes
    assert run.best.stop_reason == "revision"
    assert run.report_file.read_text(encoding="utf-8").startswith("# Revision report")
    total = json.loads((run.iterations[0].dir / "llm_response.json").read_text("utf-8"))["total"]
    print(f"\nrun: {run.run_dir}\nchanges: {changes}\ncost: {total['cost_usd']} USD")
