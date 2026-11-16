"""Audio input/output (ING-2, ING-3, ING-4, ING-6).

Internally every signal is a float32 array shaped (frames, channels). Stems are mono or stereo
WAV/FLAC (D-011); exports are 24-bit WAV.
"""

from __future__ import annotations

import logging
import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly

from stemwise.errors import AudioFormatError
from stemwise.project import Project

log = logging.getLogger(__name__)

_WAV_ENCODINGS = frozenset({"PCM_16", "PCM_24", "FLOAT"})
# Container -> accepted sample encodings (D-011, D-013): 16/24-bit integer PCM and 32-bit float
# for WAV, including its WAVE_FORMAT_EXTENSIBLE (WAVEX) and RF64 headers; 16/24-bit FLAC.
SUPPORTED: dict[str, frozenset[str]] = {
    "WAV": _WAV_ENCODINGS,
    "WAVEX": _WAV_ENCODINGS,
    "RF64": _WAV_ENCODINGS,
    "FLAC": frozenset({"PCM_16", "PCM_24"}),
}
MAX_CHANNELS = 2


@dataclass(frozen=True)
class AudioInfo:
    path: Path
    sample_rate: int
    channels: int
    frames: int


def probe(path: Path) -> AudioInfo:
    """Read the header of `path` and check format, encoding and channel count."""
    try:
        info = sf.info(str(path))
    except (sf.LibsndfileError, RuntimeError) as exc:
        raise AudioFormatError(f"cannot read audio file {path}: {exc}") from exc
    allowed = SUPPORTED.get(info.format)
    if allowed is None or info.subtype not in allowed:
        accepted = "; ".join(f"{fmt} {', '.join(sorted(subs))}" for fmt, subs in SUPPORTED.items())
        raise AudioFormatError(
            f"unsupported audio format {info.format}/{info.subtype} in {path} "
            f"(accepted: {accepted})"
        )
    if not 1 <= info.channels <= MAX_CHANNELS:
        raise AudioFormatError(f"{path} has {info.channels} channels: only mono or stereo")
    return AudioInfo(Path(path), info.samplerate, info.channels, info.frames)


def read_audio(path: Path) -> tuple[np.ndarray, int]:
    """Return `(audio, sample_rate)`; `audio` is float32 shaped (frames, channels)."""
    info = probe(path)
    try:
        audio, sr = sf.read(str(path), dtype="float32", always_2d=True)
    except (sf.LibsndfileError, RuntimeError) as exc:
        raise AudioFormatError(f"cannot decode audio file {path}: {exc}") from exc
    return np.ascontiguousarray(audio), int(sr) if sr else info.sample_rate


def _write_wav(path: Path, audio: np.ndarray, sample_rate: int, subtype: str) -> Path:
    path = Path(path)
    if path.exists():
        raise FileExistsError(f"refusing to overwrite {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(str(path), np.asarray(audio, dtype=np.float32), sample_rate, subtype, format="WAV")
    return path


def write_wav24(path: Path, audio: np.ndarray, sample_rate: int) -> Path:
    """Write `audio` as a 24-bit PCM WAV. Never overwrites an existing file."""
    return _write_wav(path, audio, sample_rate, "PCM_24")


def write_wav16(path: Path, audio: np.ndarray, sample_rate: int) -> Path:
    """Write `audio` as a 16-bit PCM WAV, without dither (MAS-2, D-075). Never overwrites."""
    return _write_wav(path, audio, sample_rate, "PCM_16")


def resolve_sample_rate(rates: dict[str, int], declared: int | str) -> int:
    """Common sample rate of the stems (ING-3, D-010). No resampling: any mismatch is an error."""
    distinct = sorted(set(rates.values()))
    listing = ", ".join(f"{sid} {sr} Hz" for sid, sr in rates.items())
    if len(distinct) > 1:
        raise AudioFormatError(f"stems have different sample rates ({listing}); re-export them")
    actual = distinct[0]
    if declared != "auto" and actual != declared:
        raise AudioFormatError(
            f"project.yaml declares sample_rate {declared} Hz but the stems are at {actual} Hz "
            f"({listing})"
        )
    return actual


def pad_to_longest(audio: dict[str, np.ndarray], sample_rate: int) -> dict[str, np.ndarray]:
    """Pad shorter signals with trailing silence to the longest one, with a warning (ING-4)."""
    frames = max(a.shape[0] for a in audio.values())
    if frames == 0:
        raise AudioFormatError("every stem is empty (0 samples)")
    short = {sid: a.shape[0] for sid, a in audio.items() if a.shape[0] < frames}
    if not short:
        return dict(audio)
    details = ", ".join(f"{sid} ({n / sample_rate:.3f} s)" for sid, n in short.items())
    log.warning(
        "stem durations differ, padded with trailing silence to %.3f s: %s",
        frames / sample_rate,
        details,
    )
    return {
        sid: a if a.shape[0] == frames else np.pad(a, ((0, frames - a.shape[0]), (0, 0)))
        for sid, a in audio.items()
    }


@dataclass(frozen=True)
class Stems:
    """The project's stems, decoded, same length, in project order."""

    sample_rate: int
    audio: dict[str, np.ndarray]

    @property
    def frames(self) -> int:
        return next(iter(self.audio.values())).shape[0]

    @property
    def channels(self) -> dict[str, int]:
        return {sid: a.shape[1] for sid, a in self.audio.items()}


def load_stems(project: Project) -> Stems:
    """Check every stem header first (format, rate), then decode and pad (ING-2/3/4/6)."""
    infos = {s.id: probe(project.stem_path(s)) for s in project.spec.stems}
    sample_rate = resolve_sample_rate(
        {sid: i.sample_rate for sid, i in infos.items()}, project.spec.sample_rate
    )
    audio = {sid: read_audio(info.path)[0] for sid, info in infos.items()}
    return Stems(sample_rate=sample_rate, audio=pad_to_longest(audio, sample_rate))


def load_reference(path: Path, sample_rate: int) -> np.ndarray:
    """Decode a reference mix and resample it to the project rate, for analysis only (ING-5).

    Resampling (D-012): `scipy.signal.resample_poly` with the exact rational ratio
    `sample_rate / source_rate` reduced by their gcd and its default anti-aliasing FIR
    (Kaiser window, beta 5.0). The file on disk is never modified.
    """
    audio, source_rate = read_audio(path)
    if source_rate == sample_rate:
        return audio
    g = math.gcd(sample_rate, source_rate)
    resampled = resample_poly(audio, sample_rate // g, source_rate // g, axis=0)
    return np.ascontiguousarray(resampled, dtype=np.float32)


def load_references(project: Project, sample_rate: int) -> list[np.ndarray]:
    """All references of the project, at `sample_rate` (ING-5)."""
    return [load_reference(project.reference_path(r), sample_rate) for r in project.spec.references]
