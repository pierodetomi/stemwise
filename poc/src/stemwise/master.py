"""Mastering of a mix against a reference track (MAS-1 → MAS-3, D-069 → D-076).

The master matches the mix to one reference with Matchering (MAS-2): the library brings the
mix to the RMS and the average spectrum of the reference, then applies its brickwall limiter.
It runs in memory at the project sample rate (D-070, D-071): reading, resampling and writing
stay in `audio_io`, and the checks the library would do on files are done here first, with
clear messages. Files are written by `runs.py`.
"""

from __future__ import annotations

import re
import warnings
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING, Literal

import numpy as np
from pydantic import BaseModel, ConfigDict, Field, ValidationError

from stemwise.analysis.loudness import integrated_lufs
from stemwise.analysis.mix import measure_mix
from stemwise.analysis.model import Bands, Db, dumps_document, limited_db, rounded
from stemwise.analysis.reference import measure_reference
from stemwise.analysis.stem import true_peak_db
from stemwise.audio_io import load_reference, read_audio
from stemwise.errors import ReportError, StemwiseError
from stemwise.evaluate import Best, spectral_distance
from stemwise.project import Project
from stemwise.report import find_run

if TYPE_CHECKING:
    import matchering

MASTER_ALGORITHM = "master v1"  # docs/analysis.md, section "Master"
FFT_SIZE = 4096  # matchering default; an input must be longer than this (in samples)
MAX_LENGTH_S = 15 * 60.0  # matchering default `max_length`
TRUE_PEAK_CEILING_DB = -1.0  # MAS-3: warning above this true peak, no correction (D-074)
LOUDNESS_BLOCK_S = 0.4  # BS.1770 gating block: shorter audio has no integrated loudness


class MasterError(StemwiseError):
    """The master cannot be made: bad inputs, missing reference, or no mix to start from."""


def _stereo(audio: np.ndarray, name: str) -> np.ndarray:
    """Float64 (frames, 2); a mono input is duplicated on both channels, as the library does."""
    x = np.asarray(audio, dtype=np.float64)
    if x.ndim == 1:
        x = x[:, None]
    if x.ndim != 2 or x.shape[1] not in (1, 2):
        raise MasterError(f"the {name} must be mono or stereo, got shape {x.shape}")
    return np.repeat(x, 2, axis=1) if x.shape[1] == 1 else x


def _check(audio: np.ndarray, sample_rate: int, name: str) -> None:
    frames = audio.shape[0]
    if frames <= FFT_SIZE:
        raise MasterError(
            f"the {name} is too short to master: {frames} samples, need more than {FFT_SIZE}"
        )
    if frames > MAX_LENGTH_S * sample_rate:
        raise MasterError(
            f"the {name} is longer than {MAX_LENGTH_S / 60:g} minutes: {frames / sample_rate:.1f} s"
        )
    if frames < LOUDNESS_BLOCK_S * sample_rate:
        raise MasterError(
            f"the {name} is too short to measure its loudness: {frames / sample_rate:.2f} s, "
            f"need at least {LOUDNESS_BLOCK_S} s"
        )
    lufs = integrated_lufs(audio, sample_rate)
    if lufs is None or not np.isfinite(lufs):
        raise MasterError(f"the {name} is silent: its loudness cannot be measured")


def _library():
    """Import matchering only when a master is made, so other commands do not depend on it."""
    with warnings.catch_warnings():
        # matchering 2.0.6 imports `scipy.ndimage.filters`, deprecated since SciPy 1.8.
        warnings.filterwarnings("ignore", category=DeprecationWarning, message=r".*scipy\.ndimage")
        import matchering
        from matchering import stages
    return matchering, stages


def config(sample_rate: int) -> matchering.Config:
    """D-070: the library defaults, with the internal rate set to the project rate."""
    matching, _ = _library()
    return matching.Config(internal_sample_rate=int(sample_rate))


def master_audio(mix: np.ndarray, reference: np.ndarray, sample_rate: int) -> np.ndarray:
    """MAS-2: master `mix` against `reference`, both at `sample_rate`; pure, deterministic.

    Inputs are (frames, channels) arrays, mono or stereo; the result is a float64 stereo array as
    long as the mix. Raises MasterError for inputs the library would refuse (D-071).
    """
    target = _stereo(mix, "mix")
    ref = _stereo(reference, "reference")
    _check(target, sample_rate, "mix")
    _check(ref, sample_rate, "reference")
    if target.shape == ref.shape and np.allclose(target, ref):
        raise MasterError("the mix and the reference are the same audio: nothing to match")
    _, stages = _library()
    result, _, _ = stages.main(target, ref, config(sample_rate), need_default=True)
    return np.ascontiguousarray(result, dtype=np.float64)


# --- contract of master_report.json (MAS-3, D-076) ---------------------------------------------


class _Contract(BaseModel):
    model_config = ConfigDict(extra="forbid", allow_inf_nan=False)


class MixSource(_Contract):
    """Where the mastered mix comes from."""

    run: str | None = Field(description="Run folder of the mix; null for a mix given as a file.")
    iteration: int | None = Field(ge=0, description="Iteration of the mix; null for a file.")
    file: str = Field(min_length=1, description="Path of the mix, as read.")


class MasterMix(MixSource):
    """The mix before the master, measured as the master is (for comparison)."""

    lufs: Db | None = Field(description="Integrated loudness of the mix, LUFS (BS.1770).")
    true_peak_db: Db = Field(description="True peak of the mix with 4x oversampling, dBTP.")
    spectral_distance_db: float | None = Field(
        ge=0.0, description="Distance of the mix from the reference (EVA-2 formula), dB."
    )


class ReferenceSource(_Contract):
    """The reference the mix is matched to (MAS-1)."""

    id: Literal["ref1", "ref2", "ref3"] | None = Field(
        description="Reference of project.yaml; null for a file given on the command line."
    )
    file: str = Field(min_length=1, description="Path of the reference, as given.")


class MasterReference(ReferenceSource):
    lufs: Db | None = Field(description="Integrated loudness of the reference, LUFS.")
    true_peak_db: Db = Field(description="True peak of the reference at the project rate, dBTP.")


class MasterReport(_Contract):
    """Content of master_report.json: what was mastered against what, and the result (MAS-3)."""

    version: Literal[1] = 1
    sample_rate: int = Field(gt=0, description="Sample rate of the master, Hz (D-070).")
    files: list[str] = Field(min_length=1, description="Audio files written, in the folder.")
    mix: MasterMix
    reference: MasterReference
    lufs: Db | None = Field(description="Integrated loudness of the master, LUFS (BS.1770).")
    true_peak_db: Db = Field(description="True peak of the master with 4x oversampling, dBTP.")
    true_peak_ok: bool = Field(description="True peak at or below -1 dBTP (MAS-3).")
    spectral_distance_db: float | None = Field(
        ge=0.0,
        description="Mean absolute band difference from the reference after level alignment.",
    )
    band_delta_db: Bands | None = Field(
        description="Reference minus master per band, after level alignment: positive = missing."
    )
    warnings: list[str] = Field(description="Warnings of the master, in English.")


def dumps_master_report(report: MasterReport) -> str:
    return dumps_document(report.model_dump(mode="json"))


def _opt_db(value: float | None) -> float | None:
    return None if value is None else rounded(limited_db(value))


def build_report(
    *,
    sample_rate: int,
    master: np.ndarray,
    mix: np.ndarray,
    reference: np.ndarray,
    mix_source: MixSource,
    reference_source: ReferenceSource,
    files: list[str],
) -> MasterReport:
    """MAS-3 (D-076): measure the master, the mix and the reference, all at `sample_rate`.

    `master` is the audio as written (the 24-bit file read back). Loudness and true peak are those
    of ANA-1; the distance is the formula of EVA-2 between the bands of ANA-7 of the master (and,
    for comparison, of the mix) and those of ANA-11 of the reference used. Mix and reference are
    measured as they enter the master: a mono one duplicated on two channels. `true_peak_ok`
    compares the true peak at full precision with −1 dBTP, as EVA-4 does with its ceiling.
    """
    mix = _stereo(mix, "mix")  # as the library uses them: a mono input is on both channels
    reference = _stereo(reference, "reference")
    measured = measure_mix(master, sample_rate)
    mix_measured = measure_mix(mix, sample_rate)
    ref_bands = measure_reference(reference, sample_rate).bands_db

    def distance(bands: list[float] | None) -> tuple[float | None, list[float] | None]:
        if bands is None or ref_bands is None:
            return None, None
        d, delta = spectral_distance(bands, ref_bands)
        return rounded(d), [rounded(limited_db(v)) for v in delta]

    master_distance, delta = distance(measured.bands_db)
    mix_distance, _ = distance(mix_measured.bands_db)
    tp_ok = measured.true_peak_db <= TRUE_PEAK_CEILING_DB  # full precision, as EVA-4 (D-053)
    warnings_: list[str] = []
    if not tp_ok:
        warnings_.append(
            f"true peak of the master is {measured.true_peak_db:+.2f} dBTP, above "
            f"{TRUE_PEAK_CEILING_DB:.1f} dBTP (MAS-3): kept as is, no correction"
        )
    return MasterReport(
        sample_rate=sample_rate,
        files=files,
        mix=MasterMix(
            **mix_source.model_dump(),
            lufs=_opt_db(mix_measured.lufs),
            true_peak_db=rounded(limited_db(mix_measured.true_peak_db)),
            spectral_distance_db=mix_distance,
        ),
        reference=MasterReference(
            **reference_source.model_dump(),
            lufs=_opt_db(integrated_lufs(reference, sample_rate)),
            true_peak_db=rounded(limited_db(true_peak_db(reference))),
        ),
        lufs=_opt_db(measured.lufs),
        true_peak_db=rounded(limited_db(measured.true_peak_db)),
        true_peak_ok=tp_ok,
        spectral_distance_db=master_distance,
        band_delta_db=delta,
        warnings=warnings_,
    )


# --- choice and loading of the inputs (MAS-1, D-073) -------------------------------------------

_REFERENCE_ID = re.compile(r"^ref([1-9])$")


@dataclass(frozen=True)
class ChosenMix:
    run_dir: Path
    path: Path
    source: MixSource


@dataclass(frozen=True)
class ChosenReference:
    path: Path
    source: ReferenceSource


def _label(project_root: Path, run_dir: Path) -> str:
    runs = (Path(project_root) / "runs").resolve()
    return run_dir.name if run_dir.resolve().parent == runs else str(run_dir)


def select_mix(project_root: Path, run: Path | None = None) -> ChosenMix:
    """D-073: the `mix.wav` of the best plan of the latest run with `best.json`, or of `run`."""
    try:
        run_dir = find_run(project_root, run)
    except ReportError as exc:
        raise MasterError(f"{exc}; or master a file with --mix") from exc
    best_file = run_dir / "best.json"
    if not best_file.is_file():
        raise MasterError(f"{run_dir} has no best.json: choose a mix run with --run, or use --mix")
    try:
        best = Best.model_validate_json(best_file.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, ValidationError) as exc:
        raise MasterError(f"cannot read {best_file}: {exc}") from exc
    if best.iteration is None or best.plan is None:
        raise MasterError(
            f"run {run_dir.name} has no mix within the technical constraints: "
            "choose another run with --run, or a file with --mix"
        )
    relative = Path(best.plan).parent / "mix.wav"
    path = run_dir / relative
    if not path.is_file():
        raise MasterError(f"the best mix of run {run_dir.name} is missing: {path}")
    return ChosenMix(
        run_dir=run_dir,
        path=path,
        source=MixSource(
            run=_label(project_root, run_dir), iteration=best.iteration, file=str(relative)
        ),
    )


def select_reference(project: Project, value: str | None) -> ChosenReference:
    """D-073: `ref1` by default; `value` is `ref1`…`ref3` of project.yaml or a file path."""
    references = project.spec.references
    if value is None:
        if not references:
            raise MasterError(
                "the master needs a reference (MAS-1): add one to project.yaml "
                "or give a file with --reference"
            )
        value = "ref1"
    match = _REFERENCE_ID.match(value)
    if match is not None:
        index = int(match.group(1))
        if index > len(references):
            raise MasterError(
                f"no reference {value}: project.yaml has {len(references)} references"
            )
        spec = references[index - 1]
        return ChosenReference(
            path=project.reference_path(spec),
            source=ReferenceSource(id=value, file=spec.file),
        )
    path = Path(value)
    if not path.is_file():
        raise MasterError(f"reference file not found: {value}")
    return ChosenReference(path=path, source=ReferenceSource(id=None, file=value))


def load_mix(path: Path, sample_rate: int) -> np.ndarray:
    """The mix to master, at the project rate: no silent resampling (as ING-3)."""
    audio, rate = read_audio(path)
    if rate != sample_rate:
        raise MasterError(
            f"the mix {path} is at {rate} Hz but the project is at {sample_rate} Hz: "
            "export it at the project rate"
        )
    return audio


def load_master_reference(path: Path, sample_rate: int) -> np.ndarray:
    """The reference at the project rate (ING-5, D-012); the file is never modified."""
    return load_reference(path, sample_rate)
