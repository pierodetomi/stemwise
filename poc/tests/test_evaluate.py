"""Evaluation of a rendered mix (EVA-1 → EVA-4) and the contracts of the loop (D-049 → D-054)."""

from __future__ import annotations

import json
import math

import numpy as np
import pytest
from pydantic import ValidationError

from stemwise.analysis.analyze import analyze_details
from stemwise.analysis.mix import MixMeasures, measure_mix
from stemwise.analysis.model import mix_entry
from stemwise.analysis.reference import Target, measure_reference, target_profile
from stemwise.analysis.stem import StereoMeasures, dc_offset_pct
from stemwise.audio_io import Stems
from stemwise.decision.schema import MixPlan
from stemwise.evaluate import (
    Best,
    Constraints,
    Evaluation,
    dumps_evaluation,
    evaluate_measures,
    evaluate_mix,
    spectral_distance,
)
from stemwise.render.mixbus import render_baseline
from stemwise.render.render import render_mix
from stemwise.schema_export import CONTRACTS
from tests.helpers.plan import neutral_plan
from tests.helpers.signals import SR, pink_noise, sine

MIX_BLOCK = {
    "lufs": -18.0,
    "peak_db": -3.5,
    "true_peak_db": -3.0,
    "rms_db": -17.0,
    "crest_db": 13.5,
    "dyn_range_lu": 7.0,
    "bands_db": [-30.0, -20.0, -12.0, -8.0, -5.0, -10.0, -15.0, -22.0],
    "stereo": {"correlation": 0.8, "side_mid_db": -11.0},
    "low_corr": 0.95,
}

EXAMPLE = {  # PRD §6.4, plus the `mix` block of D-049
    "iteration": 1,
    "spectral_distance_db": 1.7,
    "band_delta_db": [3.0, 1.4, -2.2, -3.0, -1.6, 0.4, 0.7, 1.3],
    "width_delta_db": -1.5,
    "dyn_delta_lu": 1.3,
    "masking_top": [{"a": "gtr_l", "b": "lead_vox", "band": "2k-4k", "score": 0.48}],
    "constraints": {"true_peak_ok": True, "low_corr_ok": True, "dc_ok": True},
    "is_best": True,
    "stop_reason": None,
    "mix": MIX_BLOCK,
}


# --- contracts (step 1) ------------------------------------------------------------------------


@pytest.mark.req("EVA-6")
def test_example_of_prd_is_valid() -> None:
    evaluation = Evaluation.model_validate(EXAMPLE)
    assert evaluation.constraints.all_ok
    assert json.loads(dumps_evaluation(evaluation)) == EXAMPLE


@pytest.mark.req("EVA-1")
def test_baseline_evaluation_has_no_iteration() -> None:
    doc = {**EXAMPLE, "iteration": None, "is_best": False}
    assert Evaluation.model_validate(doc).iteration is None


@pytest.mark.req("EVA-2")
def test_without_target_the_deltas_are_null() -> None:
    doc = {
        **EXAMPLE,
        "spectral_distance_db": None,
        "band_delta_db": None,
        "width_delta_db": None,
        "dyn_delta_lu": None,
        "stop_reason": "no_target",
    }
    assert Evaluation.model_validate(doc).spectral_distance_db is None


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("spectral_distance_db", -0.1),
        ("band_delta_db", [0.0] * 7),
        ("stop_reason", "tired"),
        ("iteration", -1),
        ("unknown", 1),
    ],
)
def test_invalid_evaluations_are_rejected(field: str, value: object) -> None:
    with pytest.raises(ValidationError):
        Evaluation.model_validate({**EXAMPLE, field: value})


def test_constraints_all_ok() -> None:
    assert not Constraints(true_peak_ok=True, low_corr_ok=False, dc_ok=True).all_ok


@pytest.mark.req("EVA-6")
def test_best_contract() -> None:
    best = Best(iteration=1, plan="iter_01/plan.json", spectral_distance_db=1.2, stop_reason=None)
    assert Best.model_validate_json(best.model_dump_json()) == best
    none = Best(iteration=None, plan=None, spectral_distance_db=None, stop_reason="max_iterations")
    assert none.iteration is None


@pytest.mark.req("EVA-6")
def test_contracts_registered() -> None:
    # The drift check itself is tests/test_project.py::test_committed_schema_matches_model.
    assert CONTRACTS["evaluation.schema.json"] is Evaluation
    assert CONTRACTS["best.schema.json"] is Best


def test_no_library_names() -> None:
    text = dumps_evaluation(Evaluation.model_validate(EXAMPLE)).lower()
    for name in ("numpy", "scipy", "librosa", "pyloudnorm", "pydantic", "pedalboard"):
        assert name not in text


# --- pure evaluation (step 2) ------------------------------------------------------------------

TARGET_BANDS = [-14.0, -9.5, -8.8, -9.9, -7.1, -12.4, -16.0, -21.5]


def _target(bands=TARGET_BANDS, side_mid=-9.5, dyn=6.8) -> Target:
    return Target(n_references=1, bands_db=list(bands), side_mid_db=side_mid, dyn_range_lu=dyn)


def _measures(bands=TARGET_BANDS, side_mid=-11.0, dyn=8.1, low_corr=0.9, tp=-3.2) -> MixMeasures:
    return MixMeasures(
        lufs=-18.0,
        peak_db=tp - 0.3,
        true_peak_db=tp,
        rms_db=-17.0,
        crest_db=13.5,
        dyn_range_lu=dyn,
        bands_db=None if bands is None else list(bands),
        stereo=StereoMeasures(correlation=0.8, side_mid_db=side_mid),
        low_corr=low_corr,
    )


@pytest.mark.req("EVA-2")
def test_spectral_distance_aligns_mean_level() -> None:
    louder = [b + 3.0 for b in TARGET_BANDS]
    distance, delta = spectral_distance(louder, TARGET_BANDS)
    assert distance == pytest.approx(0.0, abs=1e-9)  # exact arithmetic
    np.testing.assert_allclose(delta, [0.0] * 8, atol=1e-9)
    one_band = [b + (8.0 if i == 2 else 0.0) for i, b in enumerate(TARGET_BANDS)]
    distance, delta = spectral_distance(one_band, TARGET_BANDS)
    np.testing.assert_allclose(delta, [1, 1, -7, 1, 1, 1, 1, 1], atol=1e-9)
    assert distance == pytest.approx(1.75, abs=1e-9)


@pytest.mark.req("EVA-2")
def test_band_delta_positive_means_missing() -> None:
    dull = [*TARGET_BANDS[:6], -60.0, -70.0]  # almost nothing above 4 kHz
    _, delta = spectral_distance(dull, TARGET_BANDS)
    assert delta[6] > 0
    assert delta[7] > 0
    assert all(d < 0 for d in delta[:6])


@pytest.mark.req("EVA-2")
def test_distance_limits_empty_bands() -> None:
    empty = [-math.inf, *TARGET_BANDS[1:]]
    distance, delta = spectral_distance(empty, TARGET_BANDS)
    assert math.isfinite(distance)
    assert all(math.isfinite(d) for d in delta)


@pytest.mark.req("EVA-2")
def test_distance_ignores_reference_level() -> None:
    mix = pink_noise(duration_s=10.0, channels=2, seed=3)
    target = target_profile([measure_reference(mix * 0.25, SR)])
    evaluation = evaluate_measures(measure_mix(mix, SR), target, dc_pct=0.0, ceiling_db=-3.0)
    # Bands do not depend on the level: scatter < 0.05 between a reference and its -12 dB copy (M1).
    assert evaluation.spectral_distance_db <= 0.05


@pytest.mark.req("EVA-3")
def test_width_and_dynamics_deltas() -> None:
    evaluation = evaluate_measures(_measures(), _target(), dc_pct=0.0, ceiling_db=-3.0)
    assert evaluation.width_delta_db == pytest.approx(1.5)  # -9.5 - (-11.0): the mix is narrower
    assert evaluation.dyn_delta_lu == pytest.approx(-1.3)  # 6.8 - 8.1: the mix is more dynamic
    no_target = evaluate_measures(_measures(), None, dc_pct=0.0, ceiling_db=-3.0)
    assert no_target.spectral_distance_db is None
    assert no_target.band_delta_db is None
    assert no_target.width_delta_db is None
    assert no_target.dyn_delta_lu is None
    mono_ref = evaluate_measures(_measures(), _target(side_mid=None), dc_pct=0.0, ceiling_db=-3.0)
    assert mono_ref.width_delta_db is None
    assert mono_ref.dyn_delta_lu is not None


@pytest.mark.req("EVA-1")
def test_mix_block_matches_measure_mix() -> None:
    mix = pink_noise(duration_s=5.0, channels=2, seed=4)
    measures = measure_mix(mix, SR)
    evaluation = evaluate_measures(measures, None, dc_pct=0.0, ceiling_db=-3.0, iteration=2)
    assert evaluation.mix == mix_entry(measures)
    assert evaluation.iteration == 2
    assert not evaluation.is_best
    assert evaluation.stop_reason is None


@pytest.mark.req("EVA-4")
@pytest.mark.parametrize(
    ("kwargs", "expected"),
    [
        ({}, (True, True, True)),
        ({"tp": -2.99}, (False, True, True)),
        ({"tp": -3.0}, (True, True, True)),
        ({"low_corr": 0.0}, (True, False, True)),
        ({"low_corr": -0.4}, (True, False, True)),
        ({"low_corr": None}, (True, True, True)),  # no lows: nothing can cancel in mono (D-053)
    ],
)
def test_constraints_on_measures(kwargs: dict, expected: tuple[bool, bool, bool]) -> None:
    c = evaluate_measures(_measures(**kwargs), None, dc_pct=0.0, ceiling_db=-3.0).constraints
    assert (c.true_peak_ok, c.low_corr_ok, c.dc_ok) == expected


@pytest.mark.req("EVA-4")
@pytest.mark.parametrize(("dc_pct", "ok"), [(0.0, True), (0.1, True), (0.11, False), (0.5, False)])
def test_dc_constraint(dc_pct: float, ok: bool) -> None:
    c = evaluate_measures(_measures(), None, dc_pct=dc_pct, ceiling_db=-3.0).constraints
    assert c.dc_ok is ok


@pytest.mark.req("EVA-4")
def test_antiphase_bass_violates_low_correlation() -> None:
    bass = sine(60, duration_s=3.0, amplitude=0.3)
    antiphase = np.concatenate([bass, -bass], axis=1)
    measures = measure_mix(antiphase, SR)
    c = evaluate_measures(measures, None, dc_pct=0.0, ceiling_db=-3.0).constraints
    assert not c.low_corr_ok


@pytest.mark.req("EVA-4")
def test_dc_offset_of_a_mix() -> None:
    mix = np.concatenate([pink_noise(duration_s=2.0), pink_noise(duration_s=2.0, seed=1)], axis=1)
    mix[:, 1] += 0.005  # 0.5% of full scale on the right channel
    assert dc_offset_pct(mix) == pytest.approx(0.5, abs=1e-3)


# --- re-analysis of a rendered mix and residual collisions (step 3) ----------------------------


def _twins() -> Stems:
    x = pink_noise(duration_s=4.0, rms=0.2, seed=11)  # loud enough: -12 dB stays active
    return Stems(SR, {"a": x.copy(), "b": x.copy()})


def _plan_doc(**b_overrides) -> MixPlan:
    doc = neutral_plan(["a", "b"])
    doc["stems"]["b"].update(b_overrides)
    return MixPlan.model_validate(doc)


def _evaluate_render(stems: Stems, plan: MixPlan) -> Evaluation:
    mix = render_mix(stems, plan, pre_normalize=False, ceiling_db=-3.0, track_spectra=True)
    return evaluate_mix(mix.audio, SR, mix.track_spectra, None, ceiling_db=-3.0, iteration=0)


@pytest.mark.req("EVA-1", "EVA-3")
def test_residual_masking_follows_the_plan() -> None:
    stems = _twins()
    details = analyze_details(stems, {"a": "keys", "b": "keys"}, [], -3.0)
    baseline_bus = render_baseline(stems, -3.0)
    baseline = evaluate_mix(
        baseline_bus.audio, SR, details.stem_spectra, None, ceiling_db=-3.0, measures=details.mix
    )
    assert baseline.masking_top[0].score == 1.0  # identical stems (D-016)
    quieter = _evaluate_render(stems, _plan_doc(gain_db=-12.0))
    # 10^(-1.2) = 0.063; ±0.005 for the two-decimal rounding (D-021) and the STFT edges.
    assert all(c.score == pytest.approx(10 ** (-1.2), abs=0.005) for c in quieter.masking_top)
    panned = _evaluate_render(stems, _plan_doc(gain_db=-12.0, pan=0.5))
    assert [c.score for c in panned.masking_top] == [c.score for c in quieter.masking_top]


@pytest.mark.req("EVA-1")
def test_baseline_evaluation_reuses_the_analysis() -> None:
    stems = _twins()
    details = analyze_details(stems, {"a": "keys", "b": "keys"}, [], -3.0)
    bus = render_baseline(stems, -3.0)
    reused = evaluate_mix(
        bus.audio, SR, details.stem_spectra, None, ceiling_db=-3.0, measures=details.mix
    )
    fresh = evaluate_mix(bus.audio, SR, details.stem_spectra, None, ceiling_db=-3.0)
    assert reused == fresh
    assert reused.mix == details.analysis.mix


@pytest.mark.req("EVA-4")
def test_high_pass_removes_the_dc_of_a_stem() -> None:
    x = pink_noise(duration_s=3.0, rms=0.05, seed=5) + 0.005  # 0.5% of full scale
    stems = Stems(SR, {"a": x, "b": pink_noise(duration_s=3.0, rms=0.05, seed=6)})
    raw = _evaluate_render(stems, MixPlan.model_validate(neutral_plan(["a", "b"])))
    assert not raw.constraints.dc_ok
    doc = neutral_plan(["a", "b"])
    doc["stems"]["a"]["hpf_hz"] = 20.0
    filtered = _evaluate_render(stems, MixPlan.model_validate(doc))
    assert filtered.constraints.dc_ok


@pytest.mark.req("EVA-1", "EVA-3")
def test_neutral_plan_leaves_the_collisions_of_the_raw_sum() -> None:
    """Mono stems near the activity threshold: same threshold for the raw sum and the mix."""
    a = pink_noise(duration_s=4.0, rms=0.02, seed=21)  # bands near -50 dBFS once centred
    b = pink_noise(duration_s=4.0, rms=0.02, seed=22)
    stems = Stems(SR, {"a": a, "b": b})
    details = analyze_details(stems, {"a": "keys", "b": "keys"}, [], -3.0)
    bus = render_baseline(stems, -3.0)
    baseline = evaluate_mix(bus.audio, SR, details.stem_spectra, None, ceiling_db=-3.0)
    neutral = _evaluate_render(stems, MixPlan.model_validate(neutral_plan(["a", "b"])))
    assert baseline.masking_top  # the test is meaningful only with collisions
    assert neutral.masking_top == baseline.masking_top


@pytest.mark.req("EVA-2", "EVA-3")
def test_extreme_deltas_stay_within_the_contract() -> None:
    one_band = [0.0, *[-120.0] * 7]
    target = _target(bands=[-120.0, *[0.0] * 7], side_mid=120.0)
    measures = _measures(bands=one_band, side_mid=-120.0)
    evaluation = evaluate_measures(measures, target, dc_pct=0.0, ceiling_db=-3.0)
    assert all(-120.0 <= d <= 120.0 for d in evaluation.band_delta_db)
    assert evaluation.width_delta_db == 120.0
