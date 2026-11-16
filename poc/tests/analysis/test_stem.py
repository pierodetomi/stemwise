"""Per-stem measures: peaks (REN-5, D-009), levels on active frames (ANA-1), hygiene (ANA-6)."""

from __future__ import annotations

import math
from pathlib import Path

import numpy as np
import pytest

from stemwise.analysis.spectral import analyze_spectrum
from stemwise.analysis.stem import (
    StemMeasures,
    clipped_samples,
    crest_db,
    dc_offset_pct,
    measure_stem,
    noise_floor_db,
    onset_samples,
    onsets_per_s,
    rms_db,
    sample_peak_db,
    true_peak_db,
)
from stemwise.audio_io import read_audio
from tests.helpers.project import write_audio
from tests.helpers.signals import SR, clicks, fade, noise, noise_bursts, pink_noise, silence, sine

# ±0.1 dB: the passband ripple of the 4x interpolation FIR (Kaiser, beta 5, 81 taps) is ~0.02 dB at
# half the original Nyquist and below; the margin also covers the discrete 4x grid.
TOL_DB = 0.1


@pytest.mark.req("REN-5")
def test_true_peak_intersample() -> None:
    # fs/4 at 45°: every sample is ±sin(45°) = ±0.707 (-3.01 dBFS) but the waveform peaks at 1.0.
    x = fade(sine(SR / 4, amplitude=1.0, phase=math.pi / 4))
    assert sample_peak_db(x) == pytest.approx(20 * math.log10(math.sqrt(0.5)), abs=0.01)
    assert true_peak_db(x) == pytest.approx(0.0, abs=TOL_DB)


@pytest.mark.req("REN-5")
def test_true_peak_low_frequency() -> None:
    x = fade(sine(997, amplitude=0.5))
    assert true_peak_db(x) == pytest.approx(20 * math.log10(0.5), abs=TOL_DB)


@pytest.mark.req("REN-5")
def test_true_peak_is_max_over_channels() -> None:
    x = np.hstack([fade(sine(997, amplitude=0.25)), fade(sine(997, amplitude=0.5))])
    assert true_peak_db(x) == pytest.approx(20 * math.log10(0.5), abs=TOL_DB)


@pytest.mark.req("REN-5")
def test_true_peak_never_below_sample_peak() -> None:
    x = noise(amplitude=0.3, channels=2, seed=11)
    assert true_peak_db(x) >= sample_peak_db(x)


@pytest.mark.req("REN-5")
def test_true_peak_of_silence_is_minus_infinity() -> None:
    assert true_peak_db(silence()) == -math.inf


@pytest.mark.req("REN-5")
def test_true_peak_blocks_match_single_pass() -> None:
    """Block processing (bounded memory) gives the same result as one pass over the signal."""
    x = noise(duration_s=3.0, amplitude=0.3, channels=2, seed=5)
    one_pass = true_peak_db(x, block_frames=x.shape[0])
    assert true_peak_db(x, block_frames=4096) == pytest.approx(one_pass, abs=1e-9)


# ±0.1 dB on levels over the active frames: frames straddling an on/off edge are active but only
# partly filled, which lowers the mean power by ~1% (0.04 dB) on the signals below.
TOL_LEVEL_DB = 0.1


@pytest.mark.req("ANA-1")
def test_rms_on_active_frames() -> None:
    tone = sine(997, amplitude=0.5, duration_s=10.0)
    expected = 20 * math.log10(0.5 / math.sqrt(2))  # -9.03 dBFS
    assert rms_db(analyze_spectrum(tone, SR)) == pytest.approx(expected, abs=TOL_LEVEL_DB)
    # Half silence: the RMS of the active part is unchanged (over the whole file: -12.04 dBFS).
    gapped = np.concatenate([tone, silence(duration_s=10.0)])
    assert rms_db(analyze_spectrum(gapped, SR)) == pytest.approx(expected, abs=TOL_LEVEL_DB)
    assert rms_db(analyze_spectrum(silence(duration_s=2.0), SR)) is None


@pytest.mark.req("ANA-1")
def test_crest_factor() -> None:
    tone = sine(997, amplitude=0.5, duration_s=5.0)
    rms = rms_db(analyze_spectrum(tone, SR))
    assert crest_db(sample_peak_db(tone), rms) == pytest.approx(3.01, abs=TOL_LEVEL_DB)
    square = (0.5 * np.sign(sine(100, amplitude=1.0, phase=0.1, duration_s=5.0))).astype(np.float32)
    rms = rms_db(analyze_spectrum(square, SR))
    assert crest_db(sample_peak_db(square), rms) == pytest.approx(0.0, abs=TOL_LEVEL_DB)
    assert crest_db(-6.0, None) is None


@pytest.mark.req("ANA-6")
def test_dc_offset() -> None:
    x = sine(997, amplitude=0.5, duration_s=10.0, channels=2)
    x[:, 1] += 0.01
    # Mean of a 997 Hz sine over 10 s is ~1e-5 of full scale: ±0.01 points is ample.
    assert dc_offset_pct(x) == pytest.approx(1.0, abs=0.01)
    assert dc_offset_pct(sine(997, duration_s=10.0)) == pytest.approx(0.0, abs=0.01)


@pytest.mark.req("ANA-6")
def test_clipped_samples(tmp_path: Path) -> None:
    x = sine(997, amplitude=0.5, duration_s=1.0, channels=2)
    x[100:137, 0] = 1.0
    x[500:513, 1] = -1.0
    assert clipped_samples(x) == 50
    over = x.copy()
    over[1000:1003, 0] = 1.3  # float over
    assert clipped_samples(over) == 53
    # Full-scale PCM 16 read back: +32767 -> 1 - 2^-15, -32768 -> -1.
    pcm = read_audio(write_audio(tmp_path / "clip.wav", x, subtype="PCM_16"))[0]
    assert clipped_samples(pcm) == 50
    assert clipped_samples(sine(997, amplitude=0.99, duration_s=1.0)) == 0


def _bursts(floor: np.ndarray) -> np.ndarray:
    """Five 1 s bursts of a sine at -10 dBFS RMS, each followed by 1 s of `floor`."""
    tone = sine(997, amplitude=math.sqrt(2) * 10 ** (-10 / 20), duration_s=1.0)
    gap = floor[: tone.shape[0]]
    return np.concatenate([np.concatenate([tone + gap, gap]) for _ in range(5)])


@pytest.mark.req("ANA-6")
def test_noise_floor_white_noise() -> None:
    hiss = noise(duration_s=1.0, amplitude=10 ** (-70 / 20))  # σ = RMS = -70 dBFS
    # ±1 dB: median over ~4 s of inactive frames; edge frames between -70 and -50 dB are few.
    assert noise_floor_db(analyze_spectrum(_bursts(hiss), SR)) == pytest.approx(-70.0, abs=1.0)


@pytest.mark.req("ANA-6")
def test_noise_floor_digital_silence_and_always_active() -> None:
    assert noise_floor_db(analyze_spectrum(_bursts(silence(duration_s=1.0)), SR)) == -math.inf
    assert noise_floor_db(analyze_spectrum(sine(997, duration_s=5.0), SR)) is None
    # Less than 1 s of inactive time: not enough to estimate a floor.
    short_gap = np.concatenate([sine(997, duration_s=5.0), silence(duration_s=0.5)])
    assert noise_floor_db(analyze_spectrum(short_gap, SR)) is None


def _bed(duration_s: float) -> np.ndarray:
    """Sustained 220 Hz sine at -30 dBFS RMS: keeps every frame active, adds no onsets."""
    return sine(220, amplitude=math.sqrt(2) * 10 ** (-30 / 20), duration_s=duration_s)


def _onsets(x: np.ndarray) -> float:
    return onsets_per_s(x, SR, analyze_spectrum(x, SR))


@pytest.mark.req("ANA-4")
def test_onset_density() -> None:
    # 40 clicks in 10 s of active time; ±0.2/s = ±2 onsets (edges, peak-picking wait).
    assert _onsets(_bed(10.0) + clicks(4, duration_s=10.0)) == pytest.approx(4.0, abs=0.2)
    assert _onsets(silence(duration_s=5.0)) == 0.0


@pytest.mark.req("ANA-4")
def test_onset_density_per_active_second() -> None:
    # Same 5 s of clicks, then 5 s of silence: density is per active second (D-014). Measured
    # 4.16: the abrupt stop of the bed adds one onset (21 in 5.04 active seconds).
    active = _bed(5.0) + clicks(4, duration_s=5.0)
    x = np.concatenate([active, silence(duration_s=5.0)])
    assert _onsets(x) == pytest.approx(4.0, abs=0.3)


@pytest.mark.req("ANA-4")
def test_onsets_in_inactive_pauses_are_ignored() -> None:
    # D-023: 5 s of sustained "voice", then 15 s of headphone-bleed hits below -50 dBFS (frames
    # peak at -60 dBFS). Without the gate: 61 onsets / 5 active s = 12/s.
    x = np.concatenate(
        [
            sine(300, amplitude=0.2, duration_s=5.0),
            noise_bursts(4, duration_s=15.0, amplitude=0.003),
        ]
    )
    assert len(onset_samples(x, SR)) > 50  # precondition: the bleed is detected as onsets
    # Measured 0.2: one hit right after the voice falls in its last active frame.
    assert _onsets(x) <= 0.4


@pytest.mark.req("ANA-4")
def test_onsets_survive_anti_phase() -> None:
    # D-023: envelopes are averaged per channel, so L = -R keeps its onsets.
    mono = _bed(10.0) + clicks(4, duration_s=10.0)
    assert _onsets(np.concatenate([mono, -mono], axis=1)) == pytest.approx(4.0, abs=0.2)


@pytest.mark.req("ANA-4")
@pytest.mark.parametrize(
    "x",
    [_bed(10.0), noise(duration_s=10.0), pink_noise(duration_s=10.0)],
    ids=["sine", "white", "pink"],
)
def test_sustained_signals_have_few_onsets(x: np.ndarray) -> None:
    # D-020: with librosa's default normalization these read 26, 10 and 11 onsets/s.
    assert _onsets(x) <= 0.2


def _stereo(left: np.ndarray, right: np.ndarray) -> np.ndarray:
    return np.concatenate([left, right], axis=1)


_THETA = math.pi / 6  # 30°: L = x·cos θ, R = x·sin θ
_PAN_SIDE_MID = 20 * math.log10(
    abs(math.cos(_THETA) - math.sin(_THETA)) / (math.cos(_THETA) + math.sin(_THETA))
)


@pytest.mark.req("ANA-5")
@pytest.mark.parametrize(
    ("left_gain", "right_gain", "corr", "side_mid"),
    [
        (1.0, 1.0, 1.0, -math.inf),
        (1.0, -1.0, -1.0, math.inf),
        (math.cos(_THETA), math.sin(_THETA), 1.0, _PAN_SIDE_MID),
        (1.0, 0.0, 0.0, 0.0),
    ],
    ids=["dual-mono", "anti-phase", "panned-30deg", "left-only"],
)
def test_stereo_measures_exact(
    left_gain: float, right_gain: float, corr: float, side_mid: float
) -> None:
    x = pink_noise(duration_s=5.0)
    m = measure_stem(_stereo(left_gain * x, right_gain * x), SR).stereo
    assert m is not None
    assert m.correlation == pytest.approx(corr, abs=0.001)
    assert m.side_mid_db == pytest.approx(side_mid, abs=0.01)


@pytest.mark.req("ANA-5")
def test_stereo_measures_independent_noise() -> None:
    x = noise(duration_s=10.0, channels=2, seed=3)
    m = measure_stem(x, SR).stereo
    assert m is not None
    # 480 k samples per channel: the correlation estimate has std ~0.002; ±0.02 is ample.
    assert m.correlation == pytest.approx(0.0, abs=0.02)
    assert m.side_mid_db == pytest.approx(0.0, abs=0.1)


@pytest.mark.req("ANA-5")
def test_mono_stem_has_no_stereo() -> None:
    assert measure_stem(pink_noise(duration_s=5.0), SR).stereo is None
    silent = measure_stem(silence(duration_s=2.0, channels=2), SR).stereo
    assert silent is not None
    assert (silent.correlation, silent.side_mid_db) == (None, None)


@pytest.mark.req("ANA-1", "ANA-2", "ANA-3", "ANA-4", "ANA-6")
def test_measure_stem_collects_all_measures() -> None:
    x = np.concatenate([_bed(10.0) + clicks(4, duration_s=10.0), silence(duration_s=5.0)])
    m = measure_stem(x, SR)
    assert isinstance(m, StemMeasures)
    assert m.channels == 1
    assert m.lufs is not None
    assert m.lufs < -20
    assert m.peak_db == pytest.approx(sample_peak_db(x))
    assert m.true_peak_db >= m.peak_db
    assert m.rms_db is not None
    assert m.crest_db == pytest.approx(m.peak_db - m.rms_db)
    assert m.dyn_range_lu is not None
    assert m.active_pct == pytest.approx(100 * 10 / 15, abs=2.0)
    assert m.bands_db is not None
    assert len(m.bands_db) == 8
    assert m.centroid_hz is not None
    assert m.onsets_per_s == pytest.approx(4.0, abs=0.3)
    assert m.stereo is None
    assert m.hygiene.clipped_samples == 0
    assert m.hygiene.noise_floor_db == -math.inf


def test_measure_stem_silence() -> None:
    m = measure_stem(silence(duration_s=3.0), SR)
    assert (m.rms_db, m.crest_db, m.bands_db, m.centroid_hz, m.dyn_range_lu) == (None,) * 5
    assert (m.active_pct, m.onsets_per_s, m.lufs) == (0.0, 0.0, -math.inf)
