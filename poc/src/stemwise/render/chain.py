"""Per-stem chain (REN-1, REN-2): pre-gain, filters, EQ, compressor, makeup, gain, pan.

Every function is pure: in-memory float32 arrays (frames, channels) in, float32 out. A block that
is off leaves the signal untouched, sample for sample. Formulas: docs/analysis.md, "Render".
"""

from __future__ import annotations

import logging
import math

import numpy as np
import pedalboard

from stemwise.analysis.loudness import integrated_lufs
from stemwise.decision.schema import Compressor, StemPlan
from stemwise.render import biquad
from stemwise.render.pan import pan

log = logging.getLogger(__name__)

PRE_GAIN_TARGET_LUFS = -23.0
PRE_GAIN_WARN_DB = 24.0  # D-033: no cap, but a warning above this boost


def pre_gain_db(stem_id: str, audio: np.ndarray, sample_rate: int) -> float:
    """Gain that brings the stem to -23 LUFS (REN-1); 0 dB, with a warning, if it cannot be
    measured (silent: every block gated out; or shorter than one 400 ms block) (D-033)."""
    lufs = integrated_lufs(audio, sample_rate)
    if lufs is None or not math.isfinite(lufs):
        reason = "shorter than 400 ms" if lufs is None else "silent"
        log.warning("stem '%s' is %s: pre-gain 0 dB instead of -23 LUFS", stem_id, reason)
        return 0.0
    gain = PRE_GAIN_TARGET_LUFS - lufs
    if gain > PRE_GAIN_WARN_DB:
        log.warning(
            "stem '%s' is very quiet (%.1f LUFS): pre-gain %+.1f dB to reach -23 LUFS",
            stem_id,
            lufs,
            gain,
        )
    return gain


def apply_gain_db(audio: np.ndarray, gain_db: float) -> np.ndarray:
    """Multiply by `10^(gain_db/20)` in float64; 0 dB returns the input unchanged."""
    if gain_db == 0.0:
        return audio
    return (audio.astype(np.float64) * 10.0 ** (gain_db / 20.0)).astype(np.float32)


def compress(audio: np.ndarray, params: Compressor, sample_rate: int) -> np.ndarray:
    """`pedalboard.Compressor` (JUCE): peak detector per channel, no knee (D-034). The makeup
    gain is a separate stage of the chain. State is reset on every call."""
    plugin = pedalboard.Compressor(
        threshold_db=params.threshold_db,
        ratio=params.ratio,
        attack_ms=params.attack_ms,
        release_ms=params.release_ms,
    )
    # (channels, frames): pedalboard reads the smaller dimension as the channel count.
    planar = np.ascontiguousarray(audio.T, dtype=np.float32)
    out = plugin.process(planar, sample_rate, reset=True)
    return np.ascontiguousarray(out.T, dtype=np.float32)


def render_stem(
    audio: np.ndarray, plan: StemPlan, sample_rate: int, pre_gain: float = 0.0
) -> np.ndarray:
    """The chain of REN-2 in its fixed order, up to the pan; returns the stereo stem (frames, 2).

    pre-gain -> high-pass -> low-pass -> EQ bands (in plan order) -> compressor -> makeup ->
    gain -> pan. The reverb send is taken from this output (post-fader, post-pan).
    """
    x = apply_gain_db(audio, pre_gain)
    if plan.hpf_hz is not None:
        x = biquad.apply(x, biquad.highpass(plan.hpf_hz, sample_rate))
    if plan.lpf_hz is not None:
        x = biquad.apply(x, biquad.lowpass(plan.lpf_hz, sample_rate))
    for band in plan.eq_bands:
        x = biquad.apply(
            x, biquad.eq_band(band.type, band.freq_hz, band.gain_db, band.q, sample_rate)
        )
    if plan.compressor is not None:
        x = compress(x, plan.compressor, sample_rate)
        x = apply_gain_db(x, plan.compressor.makeup_db)
    x = apply_gain_db(x, plan.gain_db)
    return pan(x, plan.pan)
