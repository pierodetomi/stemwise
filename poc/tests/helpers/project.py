"""Builds synthetic projects (stems + project.yaml) in a temporary folder."""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import numpy as np
import soundfile as sf
import yaml

from tests.helpers.signals import SR, noise, sine


@dataclass
class StemFixture:
    id: str
    role: str
    data: np.ndarray
    sr: int = SR
    subtype: str = "PCM_24"
    ext: str = "wav"
    extra: dict[str, Any] = field(default_factory=dict)


def write_audio(path: Path, data: np.ndarray, sr: int = SR, subtype: str = "PCM_24") -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(path, data, sr, subtype=subtype)
    return path


def default_stems() -> list[StemFixture]:
    """Four short stems: mono and stereo, different roles, moderate levels."""
    return [
        StemFixture("kick", "kick", sine(60, amplitude=0.4)),
        StemFixture("bass", "bass", sine(110, amplitude=0.3)),
        StemFixture("gtr", "electric_guitar", noise(amplitude=0.05, channels=2, seed=1)),
        StemFixture("lead_vox", "lead_vocal", sine(1000, amplitude=0.2)),
    ]


def project_dict(stems: list[StemFixture], **overrides: Any) -> dict[str, Any]:
    doc: dict[str, Any] = {
        "name": "Synthetic song",
        "sample_rate": SR,
        "brief": {"genre": "indie rock", "mood": "warm", "notes": "Vocal in front."},
        "stems": [
            {"id": s.id, "file": f"stems/{s.id}.{s.ext}", "role": s.role, **s.extra} for s in stems
        ],
    }
    doc.update(overrides)
    return doc


def make_project(
    root: Path,
    stems: list[StemFixture] | None = None,
    references: list[tuple[str, np.ndarray, int]] | None = None,
    **overrides: Any,
) -> Path:
    """Write stems, optional references `(name, data, sr)` and project.yaml under `root`."""
    stems = default_stems() if stems is None else stems
    for s in stems:
        write_audio(root / "stems" / f"{s.id}.{s.ext}", s.data, s.sr, s.subtype)
    doc = project_dict(stems, **overrides)
    if references:
        doc.setdefault("references", [])
        for name, data, sr in references:
            write_audio(root / "references" / name, data, sr)
            doc["references"].append({"file": f"references/{name}"})
    root.mkdir(parents=True, exist_ok=True)
    (root / "project.yaml").write_text(yaml.safe_dump(doc, sort_keys=False), encoding="utf-8")
    return root
