"""Shared reverb (REN-4): 100% wet, global parameters, high-pass on the return (D-031).

The send bus (sum of the post-pan sends) goes through `pedalboard.Reverb` (JUCE, Freeverb tunings)
with `wet_level = 1`, `dry_level = 0`, `width = 1`, then the RBJ high-pass and the return gain.
The output has the same length as the input: the tail past the end is cut (D-031).
"""

from __future__ import annotations

import numpy as np
import pedalboard

from stemwise.decision.schema import Reverb
from stemwise.render import biquad
from stemwise.render.chain import apply_gain_db


def render_return(send_bus: np.ndarray, params: Reverb, sample_rate: int) -> np.ndarray:
    """Reverb return for a stereo send bus (frames, 2); float32 (frames, 2)."""
    plugin = pedalboard.Reverb(
        room_size=params.room_size,
        damping=params.damping,
        wet_level=1.0,
        dry_level=0.0,
        width=1.0,
        freeze_mode=0.0,
    )
    planar = np.ascontiguousarray(send_bus.T, dtype=np.float32)
    wet = np.ascontiguousarray(plugin.process(planar, sample_rate, reset=True).T)
    wet = biquad.apply(wet, biquad.highpass(params.return_hpf_hz, sample_rate))
    return apply_gain_db(wet, params.return_gain_db)
