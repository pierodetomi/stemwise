"""Shared analysis STFT: frame levels, activity, band powers, centroid (PRD §5.2, D-014).

One STFT per signal feeds bands (ANA-3), activity and centroid (ANA-4), stereo measures (ANA-5,
ANA-10) and masking (ANA-9). It is reduced on the fly to per-frame band powers and a few
accumulators: the full spectrogram is never kept. Formulas: docs/analysis.md, "STFT di analisi".
"""

from __future__ import annotations

import math
from dataclasses import dataclass

import numpy as np
from numpy.lib.stride_tricks import sliding_window_view

ACTIVITY_THRESHOLD_DB = -50.0
ACTIVITY_THRESHOLD_POWER = 10.0 ** (ACTIVITY_THRESHOLD_DB / 10.0)

# The 8 fixed bands of ANA-3, in Hz: a bin at frequency f belongs to a band when lo <= f < hi;
# the last band also includes its upper edge (20 kHz), capped at the Nyquist frequency.
BANDS_HZ: tuple[tuple[float, float], ...] = (
    (20.0, 60.0),
    (60.0, 120.0),
    (120.0, 250.0),
    (250.0, 500.0),
    (500.0, 2000.0),
    (2000.0, 4000.0),
    (4000.0, 8000.0),
    (8000.0, 20000.0),
)
BAND_LABELS: tuple[str, ...] = (
    "20-60",
    "60-120",
    "120-250",
    "250-500",
    "500-2k",
    "2k-4k",
    "4k-8k",
    "8k-20k",
)
N_BANDS = len(BANDS_HZ)
FULL_RANGE_HZ = (20.0, 20000.0)  # frame power and activity, centroid, full-band stereo
LOW_RANGE_HZ = (20.0, 120.0)  # range of the low-frequency correlation (ANA-10)

_REFERENCE_RATE = 48_000
_REFERENCE_FFT = 8192  # ~170 ms at 48 kHz: 5.9 Hz bins, 7 bins in the 20-60 Hz band
_CHUNK_FRAMES = 128


def fft_size(sample_rate: int) -> int:
    """`n_fft`: 8192 at 44.1/48 kHz, scaled by powers of two to keep a ~170 ms frame."""
    return _REFERENCE_FFT * 2 ** round(math.log2(sample_rate / _REFERENCE_RATE))


def hop_size(sample_rate: int) -> int:
    return fft_size(sample_rate) // 4


def frame_count(n_samples: int, n_fft: int, hop: int) -> int:
    """Frames start at sample 0 every `hop`; the last one is completed with zeros."""
    if n_samples == 0:
        return 0
    return 1 + math.ceil(max(n_samples - n_fft, 0) / hop)


def band_index(freqs: np.ndarray, sample_rate: int) -> np.ndarray:
    """Band number (0-7) of every bin, -1 for bins outside 20 Hz - 20 kHz."""
    index = np.full(freqs.shape, -1, dtype=np.int64)
    top = min(BANDS_HZ[-1][1], sample_rate / 2)
    for b, (lo, hi) in enumerate(BANDS_HZ):
        hi = min(hi, top)
        inside = (freqs >= lo) & ((freqs <= hi) if b == N_BANDS - 1 else (freqs < hi))
        index[inside] = b
    return index


def _range_mask(freqs: np.ndarray, lo: float, hi: float, inclusive_hi: bool) -> np.ndarray:
    return (freqs >= lo) & ((freqs <= hi) if inclusive_hi else (freqs < hi))


@dataclass(frozen=True)
class StereoSums:
    """Sums over the active frames and a frequency range of the bin powers of L, R, mid, side
    and of Re(L·conj(R)), with the same scaling as the bin powers (docs/analysis.md)."""

    ll: float
    rr: float
    lr: float
    mm: float
    ss: float


@dataclass(frozen=True)
class Spectrum:
    """Reduction of one signal's analysis STFT.

    `frame_power` and `band_power` are Hann-weighted mean squares averaged over channels, limited
    to 20 Hz - 20 kHz so DC and rumble never make a frame active (D-022) (full
    scale sine -> 0.5). `active_spectrum` is the per-bin power averaged over active frames.
    """

    sample_rate: int
    n_samples: int
    channels: int
    n_fft: int
    hop: int
    freqs: np.ndarray
    frame_power: np.ndarray  # (frames,)
    band_power: np.ndarray  # (frames, 8)
    active: np.ndarray  # (frames,) bool
    active_spectrum: np.ndarray  # (bins,)
    stereo_full: StereoSums | None
    stereo_low: StereoSums | None

    @property
    def n_frames(self) -> int:
        return int(self.frame_power.shape[0])

    @property
    def n_active(self) -> int:
        return int(np.count_nonzero(self.active))

    @property
    def duration_s(self) -> float:
        return self.n_samples / self.sample_rate

    @property
    def bus_band_power(self) -> np.ndarray:
        """Band power the signal brings to the stereo bus (mono x², stereo L² + R², D-008)."""
        return self.band_power * self.channels


def _frames(channel: np.ndarray, n_fft: int, hop: int, n_frames: int) -> np.ndarray:
    if n_frames == 0:
        return np.zeros((0, n_fft))
    needed = (n_frames - 1) * hop + n_fft
    padded = np.zeros(needed, dtype=np.float64)
    padded[: channel.shape[0]] = channel
    return sliding_window_view(padded, n_fft)[::hop][:n_frames]


def analyze_spectrum(audio: np.ndarray, sample_rate: int) -> Spectrum:
    """Compute the analysis STFT of `audio` (frames, channels) and reduce it (see module doc)."""
    a = np.asarray(audio)
    a = a[:, None] if a.ndim == 1 else a
    n_samples, channels = a.shape
    n_fft, hop = fft_size(sample_rate), hop_size(sample_rate)
    n_frames = frame_count(n_samples, n_fft, hop)
    freqs = np.fft.rfftfreq(n_fft, d=1.0 / sample_rate)
    bands = band_index(freqs, sample_rate)
    full = _range_mask(freqs, *FULL_RANGE_HZ, inclusive_hi=True)
    low = _range_mask(freqs, *LOW_RANGE_HZ, inclusive_hi=False)

    window = np.hanning(n_fft + 1)[:-1]  # periodic Hann
    # One-sided bin power such that the sum over all bins equals sum(w²x²) / sum(w²).
    weight = np.full(freqs.shape, 2.0)
    weight[0] = 1.0
    if n_fft % 2 == 0:
        weight[-1] = 1.0
    scale = weight / (n_fft * float(np.sum(window**2)))

    frame_power = np.zeros(n_frames)
    band_power = np.zeros((n_frames, N_BANDS))
    spectrum_sum = np.zeros(freqs.shape)
    stereo = channels == 2
    sums = {key: dict.fromkeys(("ll", "rr", "lr", "mm", "ss"), 0.0) for key in ("full", "low")}
    views = [_frames(a[:, c].astype(np.float64), n_fft, hop, n_frames) for c in range(channels)]

    for start in range(0, n_frames, _CHUNK_FRAMES):
        stop = min(start + _CHUNK_FRAMES, n_frames)
        spectra = [np.fft.rfft(v[start:stop] * window, axis=1) for v in views]
        powers = [np.abs(s) ** 2 * scale for s in spectra]
        mean_power = sum(powers) / channels  # (chunk, bins)
        frame_power[start:stop] = mean_power[:, full].sum(axis=1)  # 20 Hz - 20 kHz (D-022)
        for b in range(N_BANDS):
            band_power[start:stop, b] = mean_power[:, bands == b].sum(axis=1)
        active = frame_power[start:stop] > ACTIVITY_THRESHOLD_POWER
        if not np.any(active):
            continue
        spectrum_sum += mean_power[active].sum(axis=0)
        if stereo:
            left, right = spectra[0][active], spectra[1][active]
            terms = {
                "ll": np.abs(left) ** 2,
                "rr": np.abs(right) ** 2,
                "lr": np.real(left * np.conj(right)),
                "mm": np.abs((left + right) / 2) ** 2,
                "ss": np.abs((left - right) / 2) ** 2,
            }
            for key, mask in (("full", full), ("low", low)):
                for name, value in terms.items():
                    sums[key][name] += float(np.sum(value[:, mask] * scale[mask]))

    active_all = frame_power > ACTIVITY_THRESHOLD_POWER
    n_active = int(np.count_nonzero(active_all))
    return Spectrum(
        sample_rate=sample_rate,
        n_samples=n_samples,
        channels=channels,
        n_fft=n_fft,
        hop=hop,
        freqs=freqs,
        frame_power=frame_power,
        band_power=band_power,
        active=active_all,
        active_spectrum=spectrum_sum / n_active if n_active else spectrum_sum,
        stereo_full=StereoSums(**sums["full"]) if stereo else None,
        stereo_low=StereoSums(**sums["low"]) if stereo else None,
    )


def active_pct(spec: Spectrum) -> float:
    """Percentage of active frames (ANA-4); 0 for an empty signal."""
    return 100.0 * spec.n_active / spec.n_frames if spec.n_frames else 0.0


def active_seconds(spec: Spectrum) -> float:
    """Active time: the active fraction of the signal duration."""
    return active_pct(spec) / 100.0 * spec.duration_s


def band_levels_db(spec: Spectrum) -> list[float] | None:
    """ANA-3: energy of the 8 bands over the active frames, in dB relative to their sum.

    None when the signal has no active frames or no energy in 20 Hz - 20 kHz.
    """
    if spec.n_active == 0:
        return None
    energy = spec.band_power[spec.active].sum(axis=0)
    total = float(energy.sum())
    if total <= 0.0:
        return None
    with np.errstate(divide="ignore"):
        return [float(v) for v in 10.0 * np.log10(energy / total)]


def centroid_hz(spec: Spectrum) -> float | None:
    """ANA-4: power-weighted mean frequency of the active-frame spectrum, 20 Hz - 20 kHz."""
    if spec.n_active == 0:
        return None
    mask = _range_mask(spec.freqs, *FULL_RANGE_HZ, inclusive_hi=True)
    power = spec.active_spectrum[mask]
    total = float(power.sum())
    if total <= 0.0:
        return None
    return float(np.sum(spec.freqs[mask] * power) / total)
