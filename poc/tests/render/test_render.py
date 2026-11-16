"""Render of a whole plan (REN-1 → REN-7) and the acceptance criteria of M2 on arrays."""

from __future__ import annotations

from typing import Any

import numpy as np
import pytest

from stemwise.analysis.loudness import integrated_lufs
from stemwise.analysis.spectral import analyze_spectrum
from stemwise.analysis.stem import true_peak_db
from stemwise.audio_io import Stems
from stemwise.decision.schema import MixPlan
from stemwise.errors import PlanError
from stemwise.render.mixbus import render_baseline
from stemwise.render.render import RenderError, render_mix
from tests.helpers.plan import EXAMPLE_REVERB, EXAMPLE_STEM, example_plan, neutral_plan
from tests.helpers.signals import SR, noise, pink_noise, sine


def _stems(duration_s: float = 3.0) -> Stems:
    return Stems(
        SR,
        {
            "kick": sine(60.0, amplitude=0.4, duration_s=duration_s),
            "bass": sine(110.0, amplitude=0.3, duration_s=duration_s),
            "gtr": noise(amplitude=0.05, channels=2, seed=1, duration_s=duration_s),
            "lead_vox": pink_noise(rms=0.05, duration_s=duration_s, seed=7),
        },
    )


IDS = ["kick", "bass", "gtr", "lead_vox"]


def _plan(doc: dict[str, Any]) -> MixPlan:
    return MixPlan.model_validate(doc)


def _render(stems: Stems, doc: dict[str, Any], *, pre_normalize: bool = True, **kw: Any):
    return render_mix(stems, _plan(doc), pre_normalize=pre_normalize, ceiling_db=-3.0, **kw)


def _band_db(x: np.ndarray, freq: float) -> float:
    """Level of the FFT bin at `freq` (1 Hz resolution on 1 s), first channel."""
    seg = x[SR : 2 * SR, 0].astype(np.float64)
    spectrum = np.abs(np.fft.rfft(seg * np.hanning(seg.size)))
    return float(20.0 * np.log10(spectrum[round(freq)]))


@pytest.mark.req("REN-5", "REN-7")
def test_neutral_plan_reproduces_baseline() -> None:
    stems = _stems()
    result = _render(stems, neutral_plan(IDS), pre_normalize=False)
    baseline = render_baseline(stems, -3.0)
    np.testing.assert_array_equal(result.audio, baseline.audio)  # bit for bit (D-008)
    assert result.trim_gain_db == baseline.trim_gain_db


@pytest.mark.req("REN-1")
def test_pre_normalize_reports_pre_gains() -> None:
    stems = _stems()
    on = _render(stems, neutral_plan(IDS))
    off = _render(stems, neutral_plan(IDS), pre_normalize=False)
    assert set(on.pre_gain_db) == set(IDS)
    for sid in IDS:
        assert on.pre_gain_db[sid] == pytest.approx(-23.0 - integrated_lufs(stems.audio[sid], SR))
        assert off.pre_gain_db[sid] == 0.0


@pytest.mark.req("REN-5")
def test_mix_trimmed_to_ceiling() -> None:
    result = _render(_stems(), neutral_plan(IDS, gain_db=12.0), pre_normalize=False)
    assert result.true_peak_before_db > -3.0
    assert result.trim_gain_db < 0.0
    assert -3.02 <= true_peak_db(result.audio) <= -3.0  # margin of D-007


@pytest.mark.req("REN-5")
def test_quiet_mix_untouched() -> None:
    result = _render(_stems(), neutral_plan(IDS, gain_db=-12.0), pre_normalize=False)
    assert result.trim_gain_db == 0.0
    assert result.true_peak_db == result.true_peak_before_db < -3.0


@pytest.mark.req("REN-4")
def test_no_send_no_reverb() -> None:
    stems = _stems()
    doc = neutral_plan(IDS)
    dry = _render(stems, doc, keep_tracks=True)
    doc["reverb"] = dict(EXAMPLE_REVERB)  # reverb on, but no stem sends to it
    no_send = _render(stems, doc, keep_tracks=True)
    np.testing.assert_array_equal(no_send.audio, dry.audio)
    assert no_send.reverb_return is None
    assert no_send.exported_reverb_return() is None
    doc["reverb"] = None  # sends set, reverb off
    for sp in doc["stems"].values():
        sp["reverb_send_db"] = -10.0
    np.testing.assert_array_equal(_render(stems, doc).audio, dry.audio)


@pytest.mark.req("REN-4", "REN-6")
def test_exported_stems_sum_to_mix() -> None:
    stems = _stems()
    result = _render(stems, example_plan(IDS), keep_tracks=True)
    tracks = result.exported_tracks()
    ret = result.exported_reverb_return()
    assert list(tracks) == IDS
    assert ret is not None
    assert ret.any()
    for audio in [*tracks.values(), ret]:
        assert audio.shape == (stems.frames, 2)
    total = np.sum([t.astype(np.float64) for t in [*tracks.values(), ret]], axis=0)
    np.testing.assert_allclose(total, result.audio, atol=1e-6)  # float32 rounding only


@pytest.mark.req("REN-6")
def test_stem_named_like_the_reverb_return_is_kept() -> None:
    """Review of M2: the return lives apart from the stems, so a stem id cannot overwrite it."""
    stems = Stems(
        SR, {"kick": sine(60.0, duration_s=2.0), "reverb_return": sine(440.0, duration_s=2.0)}
    )
    result = _render(stems, example_plan(["kick", "reverb_return"]), keep_tracks=True)
    tracks = result.exported_tracks()
    ret = result.exported_reverb_return()
    assert list(tracks) == ["kick", "reverb_return"]
    assert ret is not None
    total = np.sum([t.astype(np.float64) for t in [*tracks.values(), ret]], axis=0)
    np.testing.assert_allclose(total, result.audio, atol=1e-6)


def test_too_short_stems_are_a_clear_error() -> None:
    stems = Stems(SR, {"a": np.zeros((2, 2), np.float32), "b": np.zeros((2, 1), np.float32)})
    with pytest.raises(RenderError, match="too short"):
        _render(stems, neutral_plan(["a", "b"]))


def test_tracks_not_kept_by_default() -> None:
    result = _render(_stems(), example_plan(IDS))
    assert result.tracks == {}
    assert result.reverb_return is None


@pytest.mark.req("REN-6")
def test_plan_must_fit_the_stems() -> None:
    with pytest.raises(PlanError, match="missing from the plan: lead_vox"):
        _render(_stems(), neutral_plan(IDS[:3]))


@pytest.mark.req("REN-2", "REN-4", "REN-5")
def test_full_plan_is_deterministic() -> None:
    stems = _stems()
    a = _render(stems, example_plan(IDS))
    b = _render(stems, example_plan(IDS))
    np.testing.assert_array_equal(a.audio, b.audio)
    assert a.audio.dtype == np.float32
    assert a.audio.shape == (stems.frames, 2)  # same duration as the stems (D-031)


@pytest.mark.req("REN-1", "REN-5")
@pytest.mark.acceptance("M2")
def test_acceptance_gain_plus_6_db() -> None:
    """+6 dB of gain on every stem ⇒ +6 LU on the mix, ±0.1 LU. Levels are kept away from the
    ceiling, and the test checks that the trim does not act."""
    stems = _stems(duration_s=5.0)
    doc = example_plan(IDS)
    for sp in doc["stems"].values():
        sp["gain_db"] = -8.0
    low = _render(stems, doc)
    for sp in doc["stems"].values():
        sp["gain_db"] = -2.0
    high = _render(stems, doc)
    assert low.trim_gain_db == 0.0
    assert high.trim_gain_db == 0.0
    delta = integrated_lufs(high.audio, SR) - integrated_lufs(low.audio, SR)
    assert delta == pytest.approx(6.0, abs=0.1)


@pytest.mark.req("REN-3")
@pytest.mark.acceptance("M2")
def test_acceptance_pan_hard_left() -> None:
    doc = example_plan(IDS)
    doc["reverb"] = None
    for sp in doc["stems"].values():
        sp["pan"] = -1.0
    result = _render(_stems(), doc)
    assert np.all(result.audio[:, 1] == 0.0)
    assert np.abs(result.audio[:, 0]).max() > 0.01


@pytest.mark.req("REN-2")
@pytest.mark.acceptance("M2")
def test_acceptance_highpass_attenuates() -> None:
    """50 Hz + 1 kHz, high-pass at 200 Hz: 50 Hz down ≥ 23 dB (24.1 analytic), 1 kHz within 0.1."""
    tone = sine(50.0, amplitude=0.2, duration_s=3.0) + sine(1_000.0, amplitude=0.2, duration_s=3.0)
    stems = Stems(SR, {"a": tone, "b": tone.copy()})
    open_ = _render(stems, neutral_plan(["a", "b"]), pre_normalize=False)
    cut = _render(stems, neutral_plan(["a", "b"], hpf_hz=200.0), pre_normalize=False)
    assert open_.trim_gain_db == cut.trim_gain_db == 0.0
    assert _band_db(open_.audio, 50.0) - _band_db(cut.audio, 50.0) >= 23.0
    assert abs(_band_db(open_.audio, 1_000.0) - _band_db(cut.audio, 1_000.0)) < 0.1


def test_example_stem_is_a_full_chain() -> None:
    # Guard: the acceptance tests above run with every block of the chain switched on.
    assert EXAMPLE_STEM["compressor"] is not None
    assert EXAMPLE_STEM["eq"]
    assert EXAMPLE_STEM["reverb_send_db"] is not None


@pytest.mark.req("EVA-1")
def test_track_spectra_match_exported_tracks() -> None:
    stems = _stems()
    result = _render(stems, example_plan(IDS), keep_tracks=True, track_spectra=True)
    assert list(result.track_spectra) == IDS
    for sid in IDS:
        expected = analyze_spectrum(result.tracks[sid], SR)
        np.testing.assert_allclose(
            result.track_spectra[sid].bus_band_power, expected.bus_band_power, rtol=1e-6
        )
    assert _render(stems, example_plan(IDS)).track_spectra == {}  # only on request
