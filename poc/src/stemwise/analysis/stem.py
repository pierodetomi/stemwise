"""Per-stem measures (PRD §5.2, ANA-1 → ANA-6). Formulas and parameters: docs/analysis.md.

Peak meters (also used by the bus trim, REN-5), levels on the active frames (D-014), hygiene
checks. Signals are arrays shaped (frames, channels) or (frames,), full scale = 1.0.
"""

from __future__ import annotations

import math
from dataclasses import dataclass

import librosa
import numpy as np
from scipy.signal import resample_poly

from stemwise.analysis.loudness import integrated_lufs, loudness_range_lu
from stemwise.analysis.spectral import (
    Spectrum,
    StereoSums,
    active_pct,
    active_seconds,
    analyze_spectrum,
    band_levels_db,
    centroid_hz,
)

TRUE_PEAK_OVERSAMPLING = 4
# Context kept on each side of a block: longer than the half-length of the interpolation FIR
# (10 input samples for resample_poly with up=4), so blocks reproduce a single pass exactly.
_BLOCK_CONTEXT = 32
_DEFAULT_BLOCK = 1 << 16

# A sample is clipped at or above the largest positive 16-bit value: catches full-scale PCM
# (16 and 24 bit) and float overs. See docs/analysis.md, "Igiene".
CLIP_LEVEL = 1.0 - 2.0**-15
# The noise floor is estimated only from at least this much inactive time.
NOISE_FLOOR_MIN_INACTIVE_S = 1.0
# Onset peak picking (D-020): absolute threshold on the un-normalized onset envelope, in dB of
# mean positive mel-spectral flux above the local mean. librosa's default (normalize to the
# maximum, delta 0.07) picks noise fluctuations as onsets on sustained signals.
ONSET_DELTA = 2.0


def _as_2d(audio: np.ndarray) -> np.ndarray:
    a = np.asarray(audio)
    return a[:, None] if a.ndim == 1 else a


def _db(linear: float) -> float:
    return 20.0 * math.log10(linear) if linear > 0 else -math.inf


def sample_peak_db(audio: np.ndarray) -> float:
    """Largest absolute sample value over all channels, in dBFS (-inf for silence)."""
    a = _as_2d(audio)
    return _db(float(np.max(np.abs(a)))) if a.size else -math.inf


def true_peak_db(audio: np.ndarray, *, block_frames: int = _DEFAULT_BLOCK) -> float:
    """True peak in dBTP: peak of the signal interpolated 4x, over all channels (D-009).

    Interpolation: `scipy.signal.resample_poly(x, 4, 1)` (Kaiser-windowed sinc FIR, beta 5.0,
    81 taps, cutoff at the original Nyquist), computed in float64. Outside the signal the input is
    taken as silence. The result is never below the sample peak. Processed in blocks of
    `block_frames` with overlapping context, to bound memory on long stems.
    """
    a = _as_2d(audio).astype(np.float64, copy=False)
    frames = a.shape[0]
    if frames == 0:
        return -math.inf
    peak = float(np.max(np.abs(a)))
    up = TRUE_PEAK_OVERSAMPLING
    for start in range(0, frames, block_frames):
        stop = min(start + block_frames, frames)
        lo, hi = max(0, start - _BLOCK_CONTEXT), min(frames, stop + _BLOCK_CONTEXT)
        y = resample_poly(a[lo:hi], up, 1, axis=0)
        core = y[(start - lo) * up : (stop - lo) * up]
        peak = max(peak, float(np.max(np.abs(core))))
    return _db(peak)


def rms_db(spec: Spectrum) -> float | None:
    """ANA-1: RMS over the active frames (mean of the Hann-weighted frame powers), in dBFS."""
    if spec.n_active == 0:
        return None
    return 10.0 * math.log10(float(np.mean(spec.frame_power[spec.active])))


def crest_db(peak_db: float, rms: float | None) -> float | None:
    """ANA-1: crest factor = sample peak - RMS over the active frames, in dB."""
    return None if rms is None else peak_db - rms


def dc_offset_pct(audio: np.ndarray) -> float:
    """ANA-6: largest absolute per-channel mean, in percent of full scale (D-018)."""
    a = _as_2d(audio).astype(np.float64, copy=False)
    if a.shape[0] == 0:
        return 0.0
    return 100.0 * float(np.max(np.abs(np.mean(a, axis=0))))


def clipped_samples(audio: np.ndarray) -> int:
    """ANA-6: number of samples, over all channels, with |x| >= 1 - 2^-15."""
    return int(np.count_nonzero(np.abs(_as_2d(audio)) >= CLIP_LEVEL))


def noise_floor_db(spec: Spectrum) -> float | None:
    """ANA-6: median level of the inactive frames, in dBFS (-inf for digital silence).

    None when the inactive time is shorter than NOISE_FLOOR_MIN_INACTIVE_S (nothing to estimate
    the floor from, e.g. a stem that never stops).
    """
    inactive = ~spec.active
    n_inactive = int(np.count_nonzero(inactive))
    if n_inactive == 0 or n_inactive / spec.n_frames * spec.duration_s < NOISE_FLOOR_MIN_INACTIVE_S:
        return None
    return _db_power(float(np.median(spec.frame_power[inactive])))


def _db_power(power: float) -> float:
    return 10.0 * math.log10(power) if power > 0 else -math.inf


def onset_samples(audio: np.ndarray, sample_rate: int) -> np.ndarray:
    """Onset positions (in samples) on the mean of the per-channel onset envelopes (D-020, D-023).

    Averaging the envelopes, not the channels, keeps anti-phase content (L = -R) and leaves the
    scale of the 2 dB threshold unchanged for dual mono.
    """
    a = _as_2d(audio).astype(np.float32, copy=False)
    envelope = np.mean(
        [librosa.onset.onset_strength(y=a[:, c], sr=sample_rate) for c in range(a.shape[1])],
        axis=0,
    )
    return librosa.onset.onset_detect(
        onset_envelope=envelope,
        sr=sample_rate,
        normalize=False,
        delta=ONSET_DELTA,
        units="samples",
    )


def onsets_per_s(audio: np.ndarray, sample_rate: int, spec: Spectrum) -> float:
    """ANA-4: onsets that fall in active frames, per second of active time (D-023).

    An onset at sample s belongs to the analysis frame whose centre is nearest,
    t = round((s - n_fft/2) / hop), clipped to the frame range.
    """
    seconds = active_seconds(spec)
    if seconds <= 0.0 or audio.shape[0] < 2:
        return 0.0
    onsets = onset_samples(audio, sample_rate)
    frames = np.clip(np.rint((onsets - spec.n_fft / 2) / spec.hop), 0, spec.n_frames - 1)
    return int(np.count_nonzero(spec.active[frames.astype(np.int64)])) / seconds


@dataclass(frozen=True)
class StereoMeasures:
    """ANA-5: L/R correlation and side/mid ratio (dB); None where undefined (no signal)."""

    correlation: float | None
    side_mid_db: float | None


def correlation(sums: StereoSums) -> float | None:
    """Zero-lag normalized L/R correlation; 0 when one channel is silent, None when both are."""
    if sums.ll <= 0.0 and sums.rr <= 0.0:
        return None
    if sums.ll <= 0.0 or sums.rr <= 0.0:
        return 0.0
    return float(np.clip(sums.lr / math.sqrt(sums.ll * sums.rr), -1.0, 1.0))


def side_mid_db(sums: StereoSums) -> float | None:
    """10·log10(side power / mid power): -inf for dual mono, +inf for pure anti-phase."""
    if sums.mm <= 0.0 and sums.ss <= 0.0:
        return None
    if sums.mm <= 0.0:
        return math.inf
    return _db_power(sums.ss / sums.mm)


def stereo_measures(spec: Spectrum) -> StereoMeasures | None:
    """ANA-5 over 20 Hz - 20 kHz and the active frames; None for a mono signal."""
    if spec.stereo_full is None:
        return None
    return StereoMeasures(correlation(spec.stereo_full), side_mid_db(spec.stereo_full))


@dataclass(frozen=True)
class Hygiene:
    dc_offset_pct: float
    clipped_samples: int
    noise_floor_db: float | None


@dataclass(frozen=True)
class StemMeasures:
    """ANA-1 → ANA-6 for one stem, at full precision (rounded only when written, ANA-13)."""

    channels: int
    lufs: float | None
    peak_db: float
    true_peak_db: float
    rms_db: float | None
    crest_db: float | None
    dyn_range_lu: float | None
    active_pct: float
    bands_db: list[float] | None
    centroid_hz: float | None
    onsets_per_s: float
    stereo: StereoMeasures | None
    hygiene: Hygiene


def measure_stem(audio: np.ndarray, sample_rate: int, spec: Spectrum | None = None) -> StemMeasures:
    """All per-stem measures (ANA-1 → ANA-6). `spec` may be passed to reuse an existing STFT."""
    a = _as_2d(audio)
    spec = spec if spec is not None else analyze_spectrum(a, sample_rate)
    peak = sample_peak_db(a)
    rms = rms_db(spec)
    return StemMeasures(
        channels=int(a.shape[1]),
        lufs=integrated_lufs(a, sample_rate),
        peak_db=peak,
        true_peak_db=true_peak_db(a),
        rms_db=rms,
        crest_db=crest_db(peak, rms),
        dyn_range_lu=loudness_range_lu(a, sample_rate),
        active_pct=active_pct(spec),
        bands_db=band_levels_db(spec),
        centroid_hz=centroid_hz(spec),
        onsets_per_s=onsets_per_s(a, sample_rate, spec),
        stereo=stereo_measures(spec),
        hygiene=Hygiene(
            dc_offset_pct=dc_offset_pct(a),
            clipped_samples=clipped_samples(a),
            noise_floor_db=noise_floor_db(spec),
        ),
    )
