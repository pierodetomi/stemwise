"""Messages for the model (DEC-1, DEC-3, D-041): content, history, request, privacy."""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from stemwise.analysis.analyze import analyze_loaded
from stemwise.analysis.model import dumps_analysis
from stemwise.decision.base import HistoryEntry, decision_input
from stemwise.decision.prompt import build_messages, limit_rows, system_prompt, user_message
from stemwise.decision.schema import MixPlan
from stemwise.loop import DISTANCE_THRESHOLD_DB, MIN_IMPROVEMENT_DB
from stemwise.project import load_project
from tests.helpers.decision import make_input
from tests.helpers.plan import neutral_plan
from tests.helpers.project import default_stems, make_project
from tests.helpers.signals import sine

pytestmark = pytest.mark.req("DEC-1")

IDS = ["kick", "bass", "gtr", "lead_vox"]


def _history(n: int) -> tuple[HistoryEntry, ...]:
    entries = []
    for i in range(n):
        plan = MixPlan.model_validate(
            {**neutral_plan(IDS), "iteration": i, "summary": f"plan of iteration {i}"}
        )
        entries.append(HistoryEntry(plan, {"iteration": i, "spectral_distance_db": 3.0 - i}))
    return tuple(entries)


def test_system_prompt_lists_every_limit() -> None:
    text = system_prompt(make_input())
    rows = limit_rows()
    for row in rows:
        assert f"| {' | '.join(row)} |" in text
    # Spot checks against PRD §6.3, independent of the generator.
    expected = {
        "stems.<id>.gain_db": ("-24 dB", "12 dB"),
        "stems.<id>.pan": ("-1", "1"),
        "stems.<id>.hpf_hz": ("20 Hz", "400 Hz"),
        "stems.<id>.lpf_hz": ("2000 Hz", "20000 Hz"),
        "stems.<id>.eq": ("0 bands", "4 bands"),
        "stems.<id>.eq[].freq_hz": ("20 Hz", "20000 Hz"),
        "stems.<id>.eq[].gain_db": ("-9 dB", "9 dB"),
        "stems.<id>.eq[].q": ("0.3", "8"),
        "stems.<id>.compressor.threshold_db": ("-40 dB", "0 dB"),
        "stems.<id>.compressor.ratio": ("1", "8"),
        "stems.<id>.compressor.attack_ms": ("0.1 ms", "100 ms"),
        "stems.<id>.compressor.release_ms": ("20 ms", "1000 ms"),
        "stems.<id>.compressor.makeup_db": ("0 dB", "12 dB"),
        "stems.<id>.reverb_send_db": ("-60 dB", "-6 dB"),
        "reverb.room_size": ("0", "1"),
        "reverb.damping": ("0", "1"),
        "reverb.return_hpf_hz": ("20 Hz", "800 Hz"),
        "reverb.return_gain_db": ("-24 dB", "0 dB"),
    }
    table = {r[0]: (r[1], r[2]) for r in rows}
    assert {k: table[k] for k in expected} == expected
    assert "one of: peak, low_shelf, high_shelf" in text


def test_system_prompt_states_role_chain_and_output_rules() -> None:
    text = system_prompt(make_input(iteration=2))
    for phrase in [
        "mixing engineer",
        "never hear the audio",
        "pre-gain, high-pass (12 dB/octave), low-pass",
        "-23 LUFS",
        "-3 dBTP",
        "±4 dB",
        "±3 dB",
        "physical bleed",
        "split over several stems",
        "Scores below\n  0.05 are negligible",
        "1-7 dB above the send",
        '"iteration": 2',
        "complete mix plan",
        "language of the brief",
    ]:
        assert phrase in text, phrase


def test_system_prompt_without_pre_normalization() -> None:
    inp = make_input()
    raw = system_prompt(type(inp)(**{**inp.__dict__, "pre_normalize": False}))
    assert "NOT normalised" in raw
    assert "-23 LUFS" not in raw


def test_user_message_contains_brief_stems_and_analysis() -> None:
    inp = make_input(notes={"gtr": "rhythm, doubled with gtr_r"})
    text = user_message(inp)
    assert inp.analysis_json.rstrip("\n") in text  # analysis sent exactly as written
    assert '"genre": "indie rock"' in text
    assert '"mood": "warm, intimate"' in text
    assert '"notes": "Vocal in front."' in text
    assert '{"id":"gtr","role":"electric_guitar","notes":"rhythm, doubled with gtr_r"}' in text
    assert '{"id":"kick","role":"kick"}' in text
    assert '"target": {' in text  # target profile is part of the analysis


def test_history_keeps_last_two() -> None:
    text = user_message(make_input(history=_history(3)))
    assert "## History (last 2 iterations, oldest first)" in text
    assert "plan of iteration 0" not in text
    first, second = text.index("plan of iteration 1"), text.index("plan of iteration 2")
    assert first < second
    assert '"spectral_distance_db":2.0' in text
    assert '"spectral_distance_db":1.0' in text


def test_no_history_section_without_history() -> None:
    assert "## History" not in user_message(make_input())


def test_request_included_only_when_given() -> None:
    assert "## Author's request" not in user_message(make_input())
    text = user_message(make_input(request="  voce più avanti, meno riverbero sulle chitarre "))
    assert (
        "## Author's request (top priority)\n\nvoce più avanti, meno riverbero sulle chitarre"
        in (text)
    )


def test_build_messages() -> None:
    inp = make_input()
    messages = build_messages(inp)
    assert [m["role"] for m in messages] == ["system", "user"]
    assert messages[0]["content"] == system_prompt(inp)
    assert messages[1]["content"] == user_message(inp)


def test_prompt_has_no_paths_title_or_audio(tmp_path: Path) -> None:
    """PRD §7: only measures, ids, roles, notes and brief; no file names, title or samples."""
    stems = default_stems()
    stems[1].extra = {"notes": "fingered bass"}
    root = make_project(
        tmp_path / "Secret Song Folder",
        stems,
        references=[("Famous Artist - Hit.wav", sine(440, amplitude=0.3, channels=2), 48_000)],
        name="My Unreleased Title",
    )
    project = load_project(root)
    analysis = analyze_loaded(project)  # the real analysis, with the reference measured
    assert analysis.references
    inp = decision_input(project, analysis, dumps_analysis(analysis))
    text = json.dumps(build_messages(inp), ensure_ascii=False)
    assert "fingered bass" in text  # the author's notes are sent (D-041)
    for secret in [
        "My Unreleased Title",
        "Secret Song Folder",
        "Famous Artist",
        "Hit.wav",
        "stems/",
        ".wav",
        str(tmp_path),
    ]:
        assert secret not in text, secret
    assert len(text) < 200_000  # no sample data


@pytest.mark.req("DEC-1", "EVA-5")
def test_system_prompt_explains_the_evaluation() -> None:
    text = system_prompt(make_input())
    assert "## Verification loop" in text
    for field in (
        "spectral_distance_db",
        "band_delta_db",
        "width_delta_db",
        "dyn_delta_lu",
        "masking_top",
        "constraints",
        "is_best",
    ):
        assert f"`{field}`" in text
    assert f"below {DISTANCE_THRESHOLD_DB:g} dB" in text
    assert f"less than {MIN_IMPROVEMENT_DB:g} dB" in text
    assert "positive means the mix lacks" in text


# --- revision (REV-1, D-062) --------------------------------------------------------------------


@pytest.mark.req("REV-1")
def test_revision_section_only_with_request() -> None:
    assert "## Revision" not in system_prompt(make_input(history=_history(1)))
    start = _history(1)
    inp = make_input(history=start, request="voce più avanti", previous_plan=start[0].plan)
    text = system_prompt(inp)
    assert "## Revision" in text
    for phrase in [
        "asked by the author after listening",
        "starting plan",
        "stems that cover",
        "no automatic\nrollback",
        "only information",
        "change\nlimits above apply from the starting plan (±4 dB of `gain_db`, ±3 dB per EQ band)",
    ]:
        assert phrase in text, phrase
    message = user_message(inp)
    assert "## Starting plan (the mix the author listened to)" in message
    assert "## History" not in message
    assert '"summary":"plan of iteration 0"' in message
    assert '"spectral_distance_db":3.0' in message
    assert message.index("## Starting plan") < message.index("## Author's request (top priority)")
    assert "Revise the starting plan as the author asks" in message
    assert "voce più avanti" in message
