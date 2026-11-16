"""Full project analysis: mix = trimmed raw sum (ANA-7, D-017), references, determinism."""

from __future__ import annotations

from pathlib import Path

import pytest

from stemwise.analysis.analyze import AnalysisError, analyze_project
from stemwise.analysis.loudness import integrated_lufs
from stemwise.analysis.mix import measure_mix
from stemwise.analysis.model import dumps_analysis, mix_entry
from stemwise.audio_io import load_stems
from stemwise.project import load_project
from stemwise.render.chain import apply_gain_db
from stemwise.render.mixbus import render_baseline, sum_bus
from tests.helpers.project import StemFixture, make_project, write_audio
from tests.helpers.signals import SR, fade, pink_noise, silence, sine


def _loud_project(root: Path, **kwargs) -> Path:
    """In-phase stems whose raw sum exceeds -3 dBTP, so the baseline is trimmed."""
    stems = [
        StemFixture("kick", "kick", fade(sine(60, amplitude=0.6, duration_s=4.0)), subtype="FLOAT"),
        StemFixture("bass", "bass", fade(sine(60, amplitude=0.6, duration_s=4.0, channels=2))),
        StemFixture("pad", "pad", pink_noise(duration_s=4.0, channels=2, rms=0.05)),
    ]
    return make_project(root, stems, **kwargs)


@pytest.mark.req("ANA-7")
def test_mix_measures_match_baseline(tmp_path: Path) -> None:
    root = _loud_project(tmp_path / "song")
    analysis = analyze_project(root)
    project = load_project(root)
    stems = load_stems(project)
    bus = render_baseline(stems, project.spec.options.mix_true_peak_db)
    assert bus.trim_gain_db < 0  # precondition: the trim is active
    assert analysis.mix == mix_entry(measure_mix(bus.audio, SR))
    untrimmed = mix_entry(measure_mix(sum_bus(stems.audio.values()), SR))
    assert analysis.mix.true_peak_db != untrimmed.true_peak_db
    assert analysis.mix.true_peak_db <= -3.0


@pytest.mark.req("ANA-11", "ANA-12")
def test_references_and_target(tmp_path: Path) -> None:
    ref = pink_noise(duration_s=4.0, channels=2, seed=8)
    root = _loud_project(
        tmp_path / "song", references=[("ref1.wav", ref, SR), ("ref2.flac", ref * 0.5, 44_100)]
    )
    analysis = analyze_project(root)
    assert [r.id for r in analysis.references] == ["ref1", "ref2"]
    assert all(r.lufs == -23.0 for r in analysis.references)
    assert analysis.target is not None
    assert analysis.target.n_references == 2
    no_refs = analyze_project(_loud_project(tmp_path / "plain"))
    assert no_refs.references == []
    assert no_refs.target is None


def test_silent_reference_is_a_clear_error(tmp_path: Path) -> None:
    root = _loud_project(
        tmp_path / "song", references=[("quiet.wav", silence(duration_s=2.0, channels=2), SR)]
    )
    with pytest.raises(AnalysisError, match=r"references/quiet\.wav"):
        analyze_project(root)


@pytest.mark.req("ANA-13")
def test_analysis_is_deterministic(tmp_path: Path) -> None:
    root = _loud_project(tmp_path / "song", references=[("r.wav", pink_noise(duration_s=3.0), SR)])
    assert dumps_analysis(analyze_project(root)) == dumps_analysis(analyze_project(root))


def test_stems_in_project_order_with_roles(tmp_path: Path) -> None:
    analysis = analyze_project(_loud_project(tmp_path / "song"))
    assert list(analysis.stems) == ["kick", "bass", "pad"]
    assert [s.role for s in analysis.stems.values()] == ["kick", "bass", "pad"]
    assert [s.channels for s in analysis.stems.values()] == [1, 2, 2]
    assert analysis.project.duration_s == 4.0
    assert list(analysis.band_share_pct) == ["kick", "bass", "pad"]


def test_reference_file_names_never_reach_the_json(tmp_path: Path) -> None:
    # D-024 (PRD §7, privacy): only measures, ids and roles go to the model.
    outside = tmp_path / "Music" / "Some Artist - Some Title.wav"
    write_audio(outside, pink_noise(duration_s=3.0, channels=2), SR)
    root = _loud_project(tmp_path / "song")
    project_yaml = root / "project.yaml"
    project_yaml.write_text(
        project_yaml.read_text(encoding="utf-8") + f"references:\n- file: {outside}\n",
        encoding="utf-8",
    )
    text = dumps_analysis(analyze_project(root))
    assert '"id": "ref1"' in text
    for leak in ("Some Artist", "Music", str(tmp_path), ".wav"):
        assert leak not in text


def _twins_project(root: Path, **overrides) -> Path:
    """The same pink noise at -10 and -30 LUFS raw: 20 dB apart, as in Q-24."""
    base = pink_noise(duration_s=4.0, channels=2, rms=0.1, seed=11)
    loud_lufs = integrated_lufs(base, SR)
    loud = apply_gain_db(base, -10.0 - loud_lufs)
    quiet = apply_gain_db(base, -30.0 - loud_lufs)
    stems = [
        StemFixture("gtr", "electric_guitar", loud, subtype="FLOAT"),
        StemFixture("vox", "lead_vocal", quiet, subtype="FLOAT"),
    ]
    return make_project(root, stems, **overrides)


@pytest.mark.req("ANA-8", "ANA-9")
def test_masking_uses_pre_normalized_stems(tmp_path: Path) -> None:
    """D-028: with pre_normalize, shares and masking are computed at -23 LUFS per stem."""
    analysis = analyze_project(_twins_project(tmp_path / "on"))
    scores = [c.score for c in analysis.masking_top]
    assert len(scores) == 8
    assert all(s == pytest.approx(1.0, abs=0.01) for s in scores)
    for share in analysis.band_share_pct["vox"]:
        assert share == pytest.approx(50.0, abs=0.1)
    # Per-stem measures stay on the raw levels.
    assert analysis.stems["gtr"].lufs == pytest.approx(-10.0, abs=0.1)
    assert analysis.stems["vox"].lufs == pytest.approx(-30.0, abs=0.1)


@pytest.mark.req("ANA-8", "ANA-9")
def test_masking_raw_without_pre_normalize(tmp_path: Path) -> None:
    root = _twins_project(tmp_path / "off", options={"pre_normalize": False})
    analysis = analyze_project(root)
    # 20 dB apart: min/max = 10^-2 ⇒ 0.01 per band; shares 1/101 ≈ 0.99 %.
    assert all(c.score == pytest.approx(0.01, abs=0.01) for c in analysis.masking_top)
    for share in analysis.band_share_pct["vox"]:
        assert share == pytest.approx(100.0 / 101.0, abs=0.1)
