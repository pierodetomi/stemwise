"""Mix plans for tests: the example of PRD §6.3 and neutral plans."""

from __future__ import annotations

import copy
import json
from pathlib import Path
from typing import Any

EXAMPLE_STEM: dict[str, Any] = {
    "gain_db": 2.0,
    "pan": 0.0,
    "hpf_hz": 90,
    "lpf_hz": None,
    "eq": [
        {"type": "peak", "freq_hz": 300, "gain_db": -2.5, "q": 1.4},
        {"type": "high_shelf", "freq_hz": 9000, "gain_db": 1.5, "q": 0.7},
    ],
    "compressor": {
        "threshold_db": -18.0,
        "ratio": 3.0,
        "attack_ms": 10.0,
        "release_ms": 120.0,
        "makeup_db": 3.0,
    },
    "reverb_send_db": -14.0,
    "rationale": "Taglio a 300 Hz per togliere la sovrapposizione con le chitarre.",
}

EXAMPLE_REVERB: dict[str, Any] = {
    "room_size": 0.35,
    "damping": 0.5,
    "return_hpf_hz": 250,
    "return_gain_db": -6.0,
}

NEUTRAL_STEM: dict[str, Any] = {
    "gain_db": 0.0,
    "pan": 0.0,
    "hpf_hz": None,
    "lpf_hz": None,
    "eq": [],
    "compressor": None,
    "reverb_send_db": None,
    "rationale": "Neutral.",
}


def example_plan(stem_ids: list[str] | None = None) -> dict[str, Any]:
    """The plan of PRD §6.3, with its `lead_vox` entry repeated for every id."""
    ids = stem_ids or ["lead_vox"]
    return {
        "version": 1,
        "iteration": 1,
        "summary": "Voce avanti, chitarre aperte ai lati, basse ripulite.",
        "reverb": copy.deepcopy(EXAMPLE_REVERB),
        "stems": {sid: copy.deepcopy(EXAMPLE_STEM) for sid in ids},
    }


def neutral_plan(stem_ids: list[str], **stem_overrides: Any) -> dict[str, Any]:
    """Gain 0, pan 0, every block off, no reverb; `stem_overrides` apply to every stem."""
    return {
        "version": 1,
        "iteration": 0,
        "summary": "Neutral plan.",
        "reverb": None,
        "stems": {sid: {**copy.deepcopy(NEUTRAL_STEM), **stem_overrides} for sid in stem_ids},
    }


def write_plan(path: Path, plan: dict[str, Any]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(plan), encoding="utf-8")
    return path
