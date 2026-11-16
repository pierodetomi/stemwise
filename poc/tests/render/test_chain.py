"""Per-stem chain (REN-1, REN-2, D-033)."""

from __future__ import annotations

import logging

import numpy as np
import pytest

from stemwise.analysis.loudness import integrated_lufs
from stemwise.decision.schema import Compressor, StemPlan
from stemwise.render import biquad
from stemwise.render.chain import apply_gain_db, compress, pre_gain_db, render_stem
from stemwise.render.mixbus import to_stereo
from stemwise.render.pan import pan
from tests.helpers.plan import EXAMPLE_STEM, NEUTRAL_STEM
from tests.helpers.signals import SR, noise, pink_noise, silence, sine


def _at_lufs(x: np.ndarray, lufs: float) -> np.ndarray:
    current = integrated_lufs(x, SR)
    assert current is not None
    return apply_gain_db(x, lufs - current)


@pytest.mark.req("REN-1")
@pytest.mark.parametrize(
    "signal",
    [
        lambda: sine(997.0, duration_s=5.0),
        lambda: pink_noise(duration_s=5.0, channels=2, seed=2),
    ],
    ids=["sine-mono", "pink-stereo"],
)
@pytest.mark.parametrize("raw_lufs", [-10.0, -35.0])
def test_pre_gain_normalizes_to_minus_23(signal, raw_lufs: float) -> None:
    x = _at_lufs(signal(), raw_lufs)
    gain = pre_gain_db("s", x, SR)
    assert gain == pytest.approx(-23.0 - raw_lufs, abs=0.1)
    out = integrated_lufs(apply_gain_db(x, gain), SR)
    assert out == pytest.approx(-23.0, abs=0.1)  # ±0.1 LU (Tech 3341); the gain itself is exact


@pytest.mark.req("REN-1")
def test_gain_zero_is_identity() -> None:
    x = sine(440.0)
    assert apply_gain_db(x, 0.0) is x  # pre_normalize false ⇒ pre-gain 0 dB ⇒ untouched


@pytest.mark.req("REN-1")
@pytest.mark.parametrize(
    ("audio", "reason"),
    [(silence(duration_s=2.0), "silent"), (sine(440.0, duration_s=0.3), "shorter than 400 ms")],
)
def test_pre_gain_silent_stem_warns(
    audio: np.ndarray, reason: str, caplog: pytest.LogCaptureFixture
) -> None:
    with caplog.at_level(logging.WARNING, logger="stemwise"):
        assert pre_gain_db("pad", audio, SR) == 0.0
    assert any(reason in r.getMessage() and "'pad'" in r.getMessage() for r in caplog.records)


@pytest.mark.req("REN-1")
def test_pre_gain_warns_on_large_boost(caplog: pytest.LogCaptureFixture) -> None:
    x = _at_lufs(sine(997.0, duration_s=3.0), -60.0)
    with caplog.at_level(logging.WARNING, logger="stemwise"):
        gain = pre_gain_db("vox", x, SR)
    assert gain == pytest.approx(37.0, abs=0.1)  # no cap (D-033)
    assert any("very quiet" in r.getMessage() for r in caplog.records)


@pytest.mark.req("REN-2")
def test_gain_db_is_exact() -> None:
    x = sine(440.0)
    np.testing.assert_allclose(apply_gain_db(x, 6.0), x * 10 ** (6 / 20), rtol=1e-6)
    np.testing.assert_allclose(apply_gain_db(x, -24.0), x * 10 ** (-24 / 20), rtol=1e-6)


def _comp(
    threshold: float, ratio: float, attack: float = 10.0, release: float = 120.0
) -> Compressor:
    return Compressor(
        threshold_db=threshold, ratio=ratio, attack_ms=attack, release_ms=release, makeup_db=0.0
    )


def _rms_db(x: np.ndarray) -> float:
    return float(10.0 * np.log10(np.mean(x.astype(np.float64) ** 2)))


@pytest.mark.req("REN-2")
def test_compressor_below_threshold() -> None:
    x = sine(1_000.0, amplitude=10 ** (-10 / 20), duration_s=2.0)  # peak -10 dBFS
    y = compress(x, _comp(-5.0, 4.0), SR)
    assert abs(_rms_db(y) - _rms_db(x)) < 0.05
    np.testing.assert_allclose(y, x, atol=1e-6)


@pytest.mark.req("REN-2")
@pytest.mark.parametrize(("attack", "release"), [(10.0, 120.0), (1.0, 100.0), (0.1, 20.0)])
def test_compressor_steady_state_reduction(attack: float, release: float) -> None:
    """Peak -10 dBFS, threshold -30, ratio 4: 20 dB over ⇒ 15 dB of reduction in steady state.
    ±1 dB: the peak detector releases between the sine peaks (measured 14.2-14.8 dB)."""
    x = sine(1_000.0, amplitude=10 ** (-10 / 20), duration_s=2.0)
    y = compress(x, _comp(-30.0, 4.0, attack, release), SR)
    steady = slice(SR, 2 * SR)
    assert _rms_db(x[steady]) - _rms_db(y[steady]) == pytest.approx(15.0, abs=1.0)


@pytest.mark.req("REN-2")
def test_compressor_ratio_one_is_identity() -> None:
    x = sine(1_000.0, amplitude=0.5, duration_s=2.0)
    y = compress(x, _comp(-30.0, 1.0), SR)
    np.testing.assert_allclose(y, x, atol=1e-6)


@pytest.mark.req("REN-2")
def test_compressor_detects_peaks() -> None:
    """Peak -10 dBFS (RMS -13): a -12 dB threshold compresses, so the detector reads peaks."""
    x = sine(1_000.0, amplitude=10 ** (-10 / 20), duration_s=2.0)
    y = compress(x, _comp(-12.0, 8.0, 1.0, 100.0), SR)
    assert _rms_db(x[SR:]) - _rms_db(y[SR:]) > 1.0


@pytest.mark.req("REN-2")
def test_compressor_channels_are_independent() -> None:
    """D-034: pedalboard's compressor is not stereo-linked; a quiet channel is left untouched."""
    loud = sine(1_000.0, amplitude=10 ** (-10 / 20), duration_s=2.0)
    x = np.concatenate([loud, loud * 0.01], axis=1)
    y = compress(x, _comp(-30.0, 4.0), SR)
    assert _rms_db(x[SR:, 0]) - _rms_db(y[SR:, 0]) == pytest.approx(15.0, abs=1.0)
    np.testing.assert_allclose(y[:, 1], x[:, 1], atol=1e-6)


def test_compressor_keeps_shape_and_is_deterministic() -> None:
    x = pink_noise(duration_s=2.0, channels=2, rms=0.2)
    a = compress(x, _comp(-30.0, 4.0), SR)
    b = compress(x, _comp(-30.0, 4.0), SR)
    assert a.shape == x.shape
    assert a.dtype == np.float32
    np.testing.assert_array_equal(a, b)


def _stem_plan(**overrides: object) -> StemPlan:
    return StemPlan.model_validate({**EXAMPLE_STEM, **overrides})


@pytest.mark.req("REN-2")
def test_chain_follows_the_fixed_order() -> None:
    """Every block on: the chain equals the stage functions composed in the order of REN-2."""
    plan = _stem_plan(lpf_hz=12_000.0, pan=-0.3)
    x = pink_noise(duration_s=2.0, rms=0.1)
    assert plan.compressor is not None
    assert plan.hpf_hz is not None
    assert plan.lpf_hz is not None
    y = apply_gain_db(x, 4.5)
    y = biquad.apply(y, biquad.highpass(plan.hpf_hz, SR))
    y = biquad.apply(y, biquad.lowpass(plan.lpf_hz, SR))
    for b in plan.eq_bands:
        y = biquad.apply(y, biquad.eq_band(b.type, b.freq_hz, b.gain_db, b.q, SR))
    y = compress(y, plan.compressor, SR)
    y = apply_gain_db(y, plan.compressor.makeup_db)
    y = apply_gain_db(y, plan.gain_db)
    expected = pan(y, plan.pan)
    np.testing.assert_array_equal(render_stem(x, plan, SR, pre_gain=4.5), expected)


@pytest.mark.req("REN-2")
def test_chain_order_gain_after_compressor() -> None:
    x = sine(1_000.0, amplitude=0.5, duration_s=2.0)
    a = render_stem(x, _stem_plan(gain_db=0.0), SR).astype(np.float64)
    b = render_stem(x, _stem_plan(gain_db=6.0), SR).astype(np.float64)
    # The compressor sees the same signal: the fader only scales its output.
    np.testing.assert_allclose(b, a * 10 ** (6 / 20), rtol=1e-6, atol=1e-9)


@pytest.mark.req("REN-1", "REN-2")
def test_chain_order_pre_gain_before_compressor() -> None:
    """With pre_normalize, the same material at -10 or -30 LUFS raw gives the same output."""
    base = pink_noise(duration_s=3.0, rms=0.1, seed=4)
    loud, quiet = _at_lufs(base, -10.0), _at_lufs(base, -30.0)
    plan = _stem_plan()
    a = render_stem(loud, plan, SR, pre_gain=pre_gain_db("x", loud, SR))
    b = render_stem(quiet, plan, SR, pre_gain=pre_gain_db("x", quiet, SR))
    np.testing.assert_allclose(a, b, atol=1e-5)  # float32 after two different gains


@pytest.mark.req("REN-2")
def test_makeup_after_compressor() -> None:
    x = sine(1_000.0, amplitude=0.5, duration_s=2.0)
    comp = dict(EXAMPLE_STEM["compressor"])
    a = render_stem(x, _stem_plan(compressor={**comp, "makeup_db": 0.0}), SR).astype(np.float64)
    b = render_stem(x, _stem_plan(compressor={**comp, "makeup_db": 3.0}), SR).astype(np.float64)
    np.testing.assert_allclose(b, a * 10 ** (3 / 20), rtol=1e-6, atol=1e-9)


@pytest.mark.req("REN-2")
def test_null_blocks_are_bypassed() -> None:
    neutral = StemPlan.model_validate(NEUTRAL_STEM)
    mono = sine(440.0)
    np.testing.assert_array_equal(render_stem(mono, neutral, SR), to_stereo(mono))
    stereo = noise(channels=2, seed=5)
    np.testing.assert_array_equal(render_stem(stereo, neutral, SR), stereo)
