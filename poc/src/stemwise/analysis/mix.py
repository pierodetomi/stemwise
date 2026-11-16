"""Mix measures (ANA-7), per-band energy shares (ANA-8), low-frequency correlation (ANA-10).

The analysed mix is the raw sum with its trim, i.e. the signal of `baseline.wav` (D-017).
Formulas: docs/analysis.md, "Mix".
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

from stemwise.analysis.loudness import integrated_lufs, loudness_range_lu
from stemwise.analysis.spectral import N_BANDS, Spectrum, analyze_spectrum, band_levels_db
from stemwise.analysis.stem import (
    StereoMeasures,
    correlation,
    crest_db,
    rms_db,
    sample_peak_db,
    stereo_measures,
    true_peak_db,
)


@dataclass(frozen=True)
class MixMeasures:
    """ANA-7 (the measures of ANA-1, ANA-2, ANA-3, ANA-5 on a stereo mix) plus ANA-10."""

    lufs: float | None
    peak_db: float
    true_peak_db: float
    rms_db: float | None
    crest_db: float | None
    dyn_range_lu: float | None
    bands_db: list[float] | None
    stereo: StereoMeasures | None
    low_corr: float | None


def low_correlation(spec: Spectrum) -> float | None:
    """ANA-10: L/R correlation over 20-120 Hz and the active frames (1.0 for a mono signal)."""
    if spec.stereo_low is None:
        return 1.0 if spec.n_active else None
    return correlation(spec.stereo_low)


def measure_mix(audio: np.ndarray, sample_rate: int, spec: Spectrum | None = None) -> MixMeasures:
    """ANA-7 and ANA-10 on a mix shaped (frames, channels)."""
    spec = spec if spec is not None else analyze_spectrum(audio, sample_rate)
    peak = sample_peak_db(audio)
    rms = rms_db(spec)
    return MixMeasures(
        lufs=integrated_lufs(audio, sample_rate),
        peak_db=peak,
        true_peak_db=true_peak_db(audio),
        rms_db=rms,
        crest_db=crest_db(peak, rms),
        dyn_range_lu=loudness_range_lu(audio, sample_rate),
        bands_db=band_levels_db(spec),
        stereo=stereo_measures(spec),
        low_corr=low_correlation(spec),
    )


def band_share_pct(stems: dict[str, Spectrum]) -> dict[str, list[float]]:
    """ANA-8 (D-017): each stem's share of the summed stem energy on the bus, per band, in %.

    Energy over all frames of the bus power (mono x², stereo L² + R²). Shares of a band sum to
    100; a band where no stem has energy gives 0 for every stem.
    """
    energy = {sid: spec.bus_band_power.sum(axis=0) for sid, spec in stems.items()}
    total = np.zeros(N_BANDS)
    for e in energy.values():
        total = total + e
    safe = np.where(total > 0, total, 1.0)
    return {
        sid: [float(v) for v in np.where(total > 0, 100.0 * e / safe, 0.0)]
        for sid, e in energy.items()
    }
