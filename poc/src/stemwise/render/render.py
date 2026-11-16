"""Render of a mix plan (REN-1 → REN-7): per-stem chains, shared reverb, mix bus with trim.

`render_mix` is a pure function of in-memory stems and a validated plan. The bus sums the
processed stems in project order and then the reverb return (float64 accumulator), and its only
processing is the trim of REN-5 (D-007). Formulas: docs/analysis.md, "Render".
"""

from __future__ import annotations

from dataclasses import dataclass, field

import numpy as np

from stemwise.analysis.spectral import Spectrum, analyze_spectrum
from stemwise.analysis.stem import true_peak_db
from stemwise.audio_io import Stems
from stemwise.decision.schema import MixPlan, check_plan_fits
from stemwise.errors import StemwiseError
from stemwise.render.chain import apply_gain_db, pre_gain_db, render_stem
from stemwise.render.mixbus import trim
from stemwise.render.reverb import render_return

MIN_RENDER_FRAMES = 3  # pedalboard reads the smaller array dimension as the channel count


class RenderError(StemwiseError):
    """The stems cannot be rendered (e.g. too short for the plugins)."""


@dataclass(frozen=True)
class MixResult:
    audio: np.ndarray  # the trimmed stereo bus, float32 (frames, 2)
    pre_gain_db: dict[str, float]
    true_peak_before_db: float
    trim_gain_db: float
    true_peak_db: float
    # Untrimmed post-pan stems and reverb return, kept only when asked for (REN-6). The return
    # is kept apart from the stems, so no stem id can collide with it.
    tracks: dict[str, np.ndarray] = field(default_factory=dict, repr=False)
    reverb_return: np.ndarray | None = field(default=None, repr=False)
    # Analysis spectra of the untrimmed post-pan stems, for the residual collisions of the
    # evaluation (EVA-1, D-054): computed one stem at a time, without keeping the tracks.
    track_spectra: dict[str, Spectrum] = field(default_factory=dict, repr=False)

    def exported_tracks(self) -> dict[str, np.ndarray]:
        """Processed stems with the bus trim; with the exported return they sum to the mix."""
        return {
            name: apply_gain_db(audio, self.trim_gain_db) for name, audio in self.tracks.items()
        }

    def exported_reverb_return(self) -> np.ndarray | None:
        if self.reverb_return is None:
            return None
        return apply_gain_db(self.reverb_return, self.trim_gain_db)


def render_mix(
    stems: Stems,
    plan: MixPlan,
    *,
    pre_normalize: bool,
    ceiling_db: float,
    keep_tracks: bool = False,
    track_spectra: bool = False,
) -> MixResult:
    """Apply `plan` to `stems` and return the trimmed mix (and, optionally, the tracks).

    With `track_spectra`, the analysis spectrum of every processed stem as it reaches the bus
    (after the pan, before the trim) is computed right after the stem is rendered.
    """
    fs = stems.sample_rate
    if stems.frames < MIN_RENDER_FRAMES:
        raise RenderError(f"stems are too short to render ({stems.frames} samples)")
    check_plan_fits(plan, list(stems.audio), fs)
    bus = np.zeros((stems.frames, 2), dtype=np.float64)
    sends: np.ndarray | None = None
    pre_gains: dict[str, float] = {}
    tracks: dict[str, np.ndarray] = {}
    spectra: dict[str, Spectrum] = {}
    ret: np.ndarray | None = None
    for sid, audio in stems.audio.items():
        sp = plan.stems[sid]
        pre_gains[sid] = pre_gain_db(sid, audio, fs) if pre_normalize else 0.0
        y = render_stem(audio, sp, fs, pre_gains[sid])
        bus += y
        if plan.reverb is not None and sp.reverb_send_db is not None:
            send = y.astype(np.float64) * 10.0 ** (sp.reverb_send_db / 20.0)
            sends = send if sends is None else sends + send
        if keep_tracks:
            tracks[sid] = y
        if track_spectra:
            spectra[sid] = analyze_spectrum(y, fs)
    if plan.reverb is not None and sends is not None:
        ret = render_return(sends.astype(np.float32), plan.reverb, fs)
        bus += ret
    mix = bus.astype(np.float32)
    before = true_peak_db(mix)
    trimmed, gain_db = trim(mix, ceiling_db)
    after = true_peak_db(trimmed) if gain_db else before
    kept_return = ret if keep_tracks else None
    return MixResult(trimmed, pre_gains, before, gain_db, after, tracks, kept_return, spectra)
