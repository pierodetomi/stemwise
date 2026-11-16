"""Mastering against a reference track (MAS-1 → MAS-3, D-069 → D-076)."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pytest
import soundfile as sf

from stemwise import master
from stemwise.analysis.loudness import gain_to_lufs, integrated_lufs
from stemwise.analysis.mix import measure_mix
from stemwise.analysis.reference import measure_reference
from stemwise.audio_io import read_audio, write_wav24
from stemwise.evaluate import spectral_distance
from stemwise.master import MasterError, master_audio
from stemwise.render.biquad import apply, high_shelf, highpass
from tests.helpers.signals import SR, pink_noise

LSB_24 = 2.0**-23  # one step of a 24-bit PCM file, on a ±1 full scale


def _at_lufs(audio: np.ndarray, lufs: float, sr: int = SR) -> np.ndarray:
    current = integrated_lufs(audio, sr)
    assert current is not None
    return (audio * gain_to_lufs(current, lufs)).astype(np.float32)


def audible_pink(*, sr: int, duration_s: float, seed: int) -> np.ndarray:
    """Stereo pink noise without its infrasonic part (4th-order high-pass at 20 Hz).

    The 1/f generator puts as much power in each octave below 20 Hz as in each audible one. The
    library matches full-band RMS, loudness ignores infrasound: with that energy in the signals
    the master came out 1.9 LU above the reference although mid RMS and spectrum matched.
    """
    x = pink_noise(sr=sr, duration_s=duration_s, channels=2, seed=seed)
    for _ in range(2):
        x = apply(x, highpass(20.0, sr))
    return x.astype(np.float32)


def dark_mix(*, sr: int = SR, duration_s: float = 10.0, lufs: float = -20.0) -> np.ndarray:
    """Pink noise with −6 dB above 2 kHz: a mix that lacks top end against a pink reference."""
    x = audible_pink(sr=sr, duration_s=duration_s, seed=1)
    x = apply(x, high_shelf(2000.0, -6.0, 0.7, sr)).astype(np.float32)
    return _at_lufs(x, lufs, sr)


def pink_reference(*, sr: int = SR, duration_s: float = 10.0, lufs: float = -14.0) -> np.ndarray:
    return _at_lufs(audible_pink(sr=sr, duration_s=duration_s, seed=2), lufs, sr)


def _distance(audio: np.ndarray, reference: np.ndarray, sr: int = SR) -> float:
    bands = measure_mix(audio, sr).bands_db
    ref_bands = measure_reference(reference, sr).bands_db
    assert bands is not None
    assert ref_bands is not None
    return spectral_distance(bands, ref_bands)[0]


@pytest.mark.req("MAS-2")
def test_matching_library_is_available_at_the_project_rate() -> None:
    """D-069, D-070: the pinned library imports and accepts an internal rate other than 44.1 kHz."""
    matchering, stages = master._library()  # silences its deprecated SciPy import

    config = matchering.Config(internal_sample_rate=48000)
    assert config.internal_sample_rate == 48000
    assert callable(stages.main)
    assert master.config(48000).internal_sample_rate == 48000  # the engine passes the rate on
    assert master.config(44100).internal_sample_rate == 44100


@pytest.mark.req("MAS-2")
def test_other_commands_do_not_import_the_library() -> None:
    """Review of M6: matchering (and its deprecated SciPy import) loads only for a master."""
    import subprocess
    import sys

    code = "import sys, stemwise.cli, stemwise.runs; print('matchering' in sys.modules)"
    out = subprocess.run([sys.executable, "-c", code], capture_output=True, text=True, check=True)
    assert out.stdout.strip() == "False"


@pytest.mark.req("MAS-2")
def test_master_moves_toward_the_reference() -> None:
    """MAS-2: the master takes the spectrum and the loudness of the reference (48 kHz, D-070)."""
    mix, reference = dark_mix(), pink_reference()
    out = master_audio(mix, reference, SR)

    before = _distance(mix, reference)
    after = _distance(out, reference)
    assert before > 2.0  # the test signal really is far from the reference
    assert after <= 0.3  # measured 0.03 dB; 0.3 = 10x margin (plan: 1.0, tightened at review)
    assert after <= before / 2

    out_lufs, ref_lufs = integrated_lufs(out, SR), integrated_lufs(reference, SR)
    assert out_lufs is not None
    assert ref_lufs is not None
    assert abs(out_lufs - ref_lufs) <= 0.3  # measured 0.02 LU; ±0.3 LU (plan: ±1, tightened)


@pytest.mark.req("MAS-2")
def test_master_keeps_rate_and_length(tmp_path: Path) -> None:
    """MAS-2, D-070: stereo master at the project rate, as long as the mix, written at 24 bit."""
    mix, reference = dark_mix(duration_s=6.0), pink_reference(duration_s=8.0)
    out = master_audio(mix, reference, SR)
    assert out.shape == (mix.shape[0], 2)
    assert np.all(np.isfinite(out))

    path = write_wav24(tmp_path / "master.wav", out, SR)
    info = sf.info(str(path))
    assert (info.samplerate, info.channels, info.frames, info.subtype) == (
        SR,
        2,
        mix.shape[0],
        "PCM_24",
    )


@pytest.mark.req("MAS-2")
def test_matches_the_library_api(tmp_path: Path) -> None:
    """D-071: the in-memory call gives the file that the documented file API writes (44.1 kHz)."""
    mg, _ = master._library()

    sr = 44100
    mix_path = write_wav24(tmp_path / "mix.wav", dark_mix(sr=sr, duration_s=6.0), sr)
    ref_path = write_wav24(tmp_path / "ref.wav", pink_reference(sr=sr, duration_s=6.0), sr)
    library_path = tmp_path / "library.wav"
    mg.process(
        target=str(mix_path),
        reference=str(ref_path),
        results=[mg.pcm24(str(library_path))],
    )

    mix, _ = read_audio(mix_path)
    reference, _ = read_audio(ref_path)
    ours_path = write_wav24(tmp_path / "ours.wav", master_audio(mix, reference, sr), sr)

    ours, _ = read_audio(ours_path)
    library, library_sr = read_audio(library_path)
    assert library_sr == sr
    assert ours.shape == library.shape
    np.testing.assert_allclose(ours, library, rtol=0, atol=LSB_24)  # 1 LSB at 24 bit


@pytest.mark.req("MAS-2")
def test_master_is_deterministic_in_memory() -> None:
    mix, reference = dark_mix(duration_s=5.0), pink_reference(duration_s=5.0)
    np.testing.assert_array_equal(
        master_audio(mix, reference, SR), master_audio(mix, reference, SR)
    )


@pytest.mark.req("MAS-1", "MAS-2")
def test_mono_inputs_give_a_stereo_master() -> None:
    mix = dark_mix(duration_s=5.0)[:, :1]
    reference = pink_reference(duration_s=5.0)[:, :1]
    out = master_audio(mix, reference, SR)
    assert out.shape == (mix.shape[0], 2)
    np.testing.assert_array_equal(out[:, 0], out[:, 1])  # mono stays centred


@pytest.mark.req("MAS-1")
@pytest.mark.parametrize(
    ("case", "message"),
    [
        ("mix_equals_reference", "same audio"),
        ("short_mix", "too short"),
        ("short_reference", "too short"),
        ("reference_under_a_loudness_block", "too short to measure its loudness"),
        ("silent_mix", "silent"),
        ("silent_reference", "silent"),
        ("long_mix", "longer than"),
        ("three_channels", "mono or stereo"),
    ],
)
def test_input_checks(case: str, message: str, monkeypatch: pytest.MonkeyPatch) -> None:
    """D-071: the checks of the library are done up front, with clear messages."""
    mix, reference = dark_mix(duration_s=3.0), pink_reference(duration_s=3.0)
    short = mix[: master.FFT_SIZE]
    silent = np.zeros_like(mix)
    if case == "mix_equals_reference":
        reference = mix.copy()
    elif case == "short_mix":
        mix = short
    elif case == "short_reference":
        reference = reference[: master.FFT_SIZE]
    elif case == "reference_under_a_loudness_block":
        reference = reference[: round(0.2 * SR)]  # 9,600 samples: longer than FFT_SIZE
    elif case == "silent_mix":
        mix = silent
    elif case == "silent_reference":
        reference = silent
    elif case == "long_mix":
        monkeypatch.setattr(master, "MAX_LENGTH_S", 2.0)
    elif case == "three_channels":
        mix = np.concatenate([mix, mix[:, :1]], axis=1)
    with pytest.raises(MasterError, match=message):
        master_audio(mix, reference, SR)


# --- measures and contract (MAS-3, D-074, D-076) ------------------------------------------------


def _report(reference: np.ndarray, *, mix: np.ndarray | None = None) -> master.MasterReport:
    mix = dark_mix(duration_s=6.0) if mix is None else mix
    out = master_audio(mix, reference, SR)
    return master.build_report(
        sample_rate=SR,
        master=out,
        mix=mix,
        reference=reference,
        mix_source=master.MixSource(run="2026-11-10T10-00-00", iteration=1, file="mix.wav"),
        reference_source=master.ReferenceSource(id="ref1", file="references/ref1.wav"),
        files=["master.wav"],
    )


@pytest.mark.req("MAS-3")
def test_master_report_measures() -> None:
    """MAS-3, D-076: the report holds the measures of ANA-1/ANA-7 and the distance of EVA-2."""
    mix, reference = dark_mix(duration_s=6.0), pink_reference(duration_s=6.0)
    out = master_audio(mix, reference, SR)
    report = _report(reference, mix=mix)

    measured = measure_mix(out, SR)
    ref_bands = measure_reference(reference, SR).bands_db
    assert ref_bands is not None
    assert measured.bands_db is not None
    distance, delta = spectral_distance(measured.bands_db, ref_bands)
    mix_bands = measure_mix(mix, SR).bands_db
    assert mix_bands is not None
    mix_distance, _ = spectral_distance(mix_bands, ref_bands)
    ref_lufs = integrated_lufs(reference, SR)
    assert ref_lufs is not None
    assert measured.lufs is not None

    assert report.lufs == round(measured.lufs, 1)
    assert report.true_peak_db == round(measured.true_peak_db, 1)
    assert report.spectral_distance_db == round(distance, 1)
    assert report.band_delta_db == [round(d, 1) + 0.0 for d in delta]
    assert report.mix.spectral_distance_db == round(mix_distance, 1)
    assert report.mix.lufs == round(integrated_lufs(mix, SR) or 0.0, 1)
    assert report.reference.lufs == round(ref_lufs, 1)
    assert report.sample_rate == SR
    assert report.files == ["master.wav"]
    assert report.mix.run == "2026-11-10T10-00-00"
    assert report.mix.iteration == 1
    assert report.reference.id == "ref1"
    # The document validates against its own contract (and so against the exported schema).
    master.MasterReport.model_validate_json(master.dumps_master_report(report))


@pytest.mark.req("MAS-3")
@pytest.mark.parametrize(("ref_lufs", "warned"), [(-9.0, True), (-30.0, False)])
def test_true_peak_warning(ref_lufs: float, warned: bool) -> None:
    """MAS-3, D-074: a warning, and no correction, when the true peak is above −1 dBTP."""
    report = _report(pink_reference(duration_s=6.0, lufs=ref_lufs))
    assert report.true_peak_ok is (not warned)
    assert (report.true_peak_db > master.TRUE_PEAK_CEILING_DB) is warned
    if warned:
        assert len(report.warnings) == 1
        assert "-1.0 dBTP" in report.warnings[0]
    else:
        assert report.warnings == []


@pytest.mark.req("MAS-3")
def test_report_is_library_neutral() -> None:
    """PRD §7.1: no library names in the exchange files."""
    from stemwise.schema_export import CONTRACTS

    assert CONTRACTS["master_report.schema.json"] is master.MasterReport
    text = master.dumps_master_report(_report(pink_reference(duration_s=6.0))).lower()
    for name in ("matchering", "pedalboard", "scipy", "numpy", "pyloudnorm", "librosa"):
        assert name not in text


# --- choice of mix and reference (MAS-1, D-073) ------------------------------------------------


def _mixed_project(root: Path, *, references: int = 1) -> Path:
    from stemwise.decision.mock import MockDecider
    from stemwise.runs import mix_run
    from tests.helpers.project import make_project

    refs = [
        (f"ref{i}.wav", pink_reference(duration_s=4.0, lufs=-14.0 - i), SR)
        for i in range(1, references + 1)
    ]
    make_project(root, references=refs or None)
    if references:
        mix_run(root, MockDecider())
    return root


def _set_best_iteration_null(run_dir: Path) -> None:
    import json

    best = json.loads((run_dir / "best.json").read_text(encoding="utf-8"))
    best.update(iteration=None, plan=None, spectral_distance_db=None)
    (run_dir / "best.json").write_text(json.dumps(best), encoding="utf-8")


@pytest.mark.req("MAS-1")
def test_mix_selection(tmp_path: Path) -> None:
    """D-073: the mix of the best plan of the latest run, or of --run; clear errors otherwise."""
    import json

    root = _mixed_project(tmp_path / "song")
    (run_dir,) = sorted((root / "runs").iterdir())
    best = json.loads((run_dir / "best.json").read_text(encoding="utf-8"))

    chosen = master.select_mix(root)
    assert chosen.run_dir == run_dir
    assert chosen.path == run_dir / Path(best["plan"]).parent / "mix.wav"
    assert chosen.source.run == run_dir.name
    assert chosen.source.iteration == best["iteration"]
    assert chosen.source.file == str(Path(best["plan"]).parent / "mix.wav")
    assert master.select_mix(root, Path(run_dir.name)).path == chosen.path  # --run by name
    assert master.select_mix(root, run_dir).path == chosen.path  # --run by path

    _set_best_iteration_null(run_dir)
    with pytest.raises(MasterError, match=r"--run.*--mix"):
        master.select_mix(root)

    empty = tmp_path / "empty"
    _mixed_project(empty, references=0)
    with pytest.raises(MasterError, match="--mix"):
        master.select_mix(empty)


@pytest.mark.req("MAS-1")
@pytest.mark.parametrize(
    ("value", "expected", "error"),
    [
        (None, ("ref1", "references/ref1.wav"), None),
        ("ref2", ("ref2", "references/ref2.wav"), None),
        ("ref3", None, "has 2 references"),
        ("outside.wav", (None, "outside.wav"), None),
        ("missing.wav", None, "not found"),
    ],
)
def test_reference_selection(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    value: str | None,
    expected: tuple[str | None, str] | None,
    error: str | None,
) -> None:
    from stemwise.project import load_project
    from tests.helpers.project import make_project

    refs = [(f"ref{i}.wav", pink_reference(duration_s=2.0), SR) for i in (1, 2)]
    project = load_project(make_project(tmp_path / "song", references=refs))
    monkeypatch.chdir(tmp_path)
    write_wav24(tmp_path / "outside.wav", pink_reference(duration_s=2.0), SR)
    if error is not None:
        with pytest.raises(MasterError, match=error):
            master.select_reference(project, value)
        return
    chosen = master.select_reference(project, value)
    assert (chosen.source.id, chosen.source.file) == expected
    assert chosen.path.is_file()


@pytest.mark.req("MAS-1")
def test_reference_is_mandatory(tmp_path: Path) -> None:
    """MAS-1: a project without references needs --reference."""
    from stemwise.project import load_project
    from tests.helpers.project import make_project

    project = load_project(make_project(tmp_path / "song"))
    with pytest.raises(MasterError, match="needs a reference"):
        master.select_reference(project, None)


@pytest.mark.req("MAS-1")
def test_input_rates(tmp_path: Path) -> None:
    """D-073: a mix at another rate is an error (as ING-3); a reference is resampled (ING-5)."""
    mix_44 = write_wav24(tmp_path / "mix44.wav", dark_mix(sr=44100, duration_s=3.0), 44100)
    with pytest.raises(MasterError, match="44100 Hz"):
        master.load_mix(mix_44, SR)

    ref_44 = write_wav24(tmp_path / "ref44.wav", pink_reference(sr=44100, duration_s=3.0), 44100)
    reference = master.load_master_reference(ref_44, SR)
    assert reference.shape[0] == 3 * SR


@pytest.mark.req("MAS-3")
def test_mono_inputs_are_measured_as_mastered() -> None:
    """Review of M6: a mono reference counts on both channels, as the library uses it."""
    reference = pink_reference(duration_s=6.0)[:, :1]
    report = _report(reference)
    assert report.lufs is not None
    assert report.reference.lufs is not None
    assert abs(report.lufs - report.reference.lufs) <= 0.3  # same level, as in the stereo case


@pytest.mark.req("MAS-3")
def test_true_peak_ok_uses_full_precision() -> None:
    """MAS-3: −0.96 dBTP is above −1 dBTP even if it is written as −1.0 (as EVA-4)."""
    from tests.helpers.signals import sine

    mix, reference = dark_mix(duration_s=3.0), pink_reference(duration_s=3.0)
    peak = 10 ** (-0.96 / 20)
    loud = sine(100.0, duration_s=3.0, amplitude=peak, channels=2)  # true peak ≈ sample peak
    report = master.build_report(
        sample_rate=SR,
        master=loud,
        mix=mix,
        reference=reference,
        mix_source=master.MixSource(run=None, iteration=None, file="mix.wav"),
        reference_source=master.ReferenceSource(id=None, file="ref.wav"),
        files=["master.wav"],
    )
    assert report.true_peak_db == -1.0  # as written, one decimal
    assert report.true_peak_ok is False
    assert len(report.warnings) == 1
    assert "above -1.0 dBTP" in report.warnings[0]
