"""Full analysis of a project: stems, mix, masking, references (ANA-1 → ANA-13).

`analyze` is a pure function of in-memory audio; `analyze_project` loads a project folder and
calls it. The analysed mix is the raw sum with its trim, the signal of `baseline.wav` (D-017).
With `pre_normalize`, band shares and masking are measured on the stems brought to -23 LUFS by
the pre-gain of REN-1, the starting point of the plan (D-028); per-stem measures stay raw.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np

from stemwise.analysis.masking import masking_top
from stemwise.analysis.mix import MixMeasures, band_share_pct, measure_mix
from stemwise.analysis.model import Analysis, StemInput, build_analysis
from stemwise.analysis.reference import (
    ReferenceLoudnessError,
    Target,
    measure_reference,
    target_profile,
)
from stemwise.analysis.spectral import Spectrum, analyze_spectrum
from stemwise.analysis.stem import measure_stem
from stemwise.audio_io import Stems, load_references, load_stems
from stemwise.errors import StemwiseError
from stemwise.project import Project, load_project
from stemwise.render.chain import apply_gain_db, pre_gain_db
from stemwise.render.mixbus import render_baseline, to_stereo


class AnalysisError(StemwiseError):
    """A file cannot be analysed (e.g. a silent reference)."""


@dataclass(frozen=True)
class ReferenceInput:
    file: str  # as written in project.yaml: used in error messages only, never in the JSON
    audio: np.ndarray  # already at the project sample rate (ING-5)


@dataclass(frozen=True)
class AnalysisDetails:
    """The contract plus the full-precision pieces the evaluation reuses (EVA-1, D-054)."""

    analysis: Analysis
    # Of the raw stems as they reach the bus of baseline.wav: mono stems centred on two channels
    # (D-008), like a rendered mono stem panned to the centre, so the activity threshold of the
    # residual collisions is the same for the raw sum and for a mix (D-054).
    stem_spectra: dict[str, Spectrum]
    mix: MixMeasures  # of the raw sum with its trim (baseline.wav)
    target: Target | None


def measure_references(references: list[ReferenceInput], fs: int) -> list[MixMeasures]:
    """ANA-11 on every reference; a silent one is an error naming its file."""
    measured = []
    for ref in references:
        try:
            measured.append(measure_reference(ref.audio, fs))
        except ReferenceLoudnessError as exc:
            raise AnalysisError(f"reference {ref.file}: {exc}") from exc
    return measured


def analyze(
    stems: Stems,
    roles: dict[str, str],
    references: list[ReferenceInput],
    mix_true_peak_db: float,
    *,
    pre_normalize: bool = False,
) -> Analysis:
    """Measure everything and assemble the `analysis.json` contract."""
    return analyze_details(
        stems, roles, references, mix_true_peak_db, pre_normalize=pre_normalize
    ).analysis


def analyze_details(
    stems: Stems,
    roles: dict[str, str],
    references: list[ReferenceInput],
    mix_true_peak_db: float,
    *,
    pre_normalize: bool = False,
) -> AnalysisDetails:
    """`analyze`, keeping the raw stem spectra, the mix measures and the target."""
    fs = stems.sample_rate
    spectra = {sid: analyze_spectrum(audio, fs) for sid, audio in stems.audio.items()}
    interaction = spectra
    if pre_normalize:  # D-028: shares and masking at the levels the plan starts from
        interaction = {
            sid: analyze_spectrum(apply_gain_db(audio, pre_gain_db(sid, audio, fs)), fs)
            for sid, audio in stems.audio.items()
        }
    measures = {
        sid: StemInput(roles[sid], measure_stem(audio, fs, spectra[sid]))
        for sid, audio in stems.audio.items()
    }
    mix = measure_mix(render_baseline(stems, mix_true_peak_db).audio, fs)
    measured_refs = measure_references(references, fs)
    target = target_profile(measured_refs)
    analysis = build_analysis(
        sample_rate=fs,
        duration_s=stems.frames / fs,
        stems=measures,
        mix=mix,
        band_share=band_share_pct(interaction),
        collisions=masking_top(interaction),
        references=measured_refs,
        target=target,
    )
    on_bus = {
        sid: spectra[sid] if audio.shape[1] == 2 else analyze_spectrum(to_stereo(audio), fs)
        for sid, audio in stems.audio.items()
    }
    return AnalysisDetails(analysis, on_bus, mix, target)


def analyze_loaded(project: Project, stems: Stems | None = None) -> Analysis:
    """Analyse a validated project; `stems` avoids decoding them again when already loaded."""
    return analyze_loaded_details(project, stems).analysis


def _reference_inputs(project: Project, sample_rate: int) -> list[ReferenceInput]:
    audio = load_references(project, sample_rate)
    return [
        ReferenceInput(spec.file, a) for spec, a in zip(project.spec.references, audio, strict=True)
    ]


def project_target(project: Project, sample_rate: int) -> Target | None:
    """ANA-12 alone: the target profile of the project's references, at full precision.

    A revision measures only the references (D-061): the stems are described by the analysis of
    the run it starts from.
    """
    return target_profile(measure_references(_reference_inputs(project, sample_rate), sample_rate))


def analyze_loaded_details(project: Project, stems: Stems | None = None) -> AnalysisDetails:
    """`analyze_loaded`, keeping what the evaluation reuses."""
    stems = load_stems(project) if stems is None else stems
    references = _reference_inputs(project, stems.sample_rate)
    roles = {s.id: s.role for s in project.spec.stems}
    options = project.spec.options
    return analyze_details(
        stems, roles, references, options.mix_true_peak_db, pre_normalize=options.pre_normalize
    )


def analyze_project(project_dir: Path) -> Analysis:
    """Load and validate `project_dir`, then analyse it."""
    return analyze_loaded(load_project(project_dir))
