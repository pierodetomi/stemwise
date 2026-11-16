"""Audio I/O: formats, sample rates, padding, internal dtype, 24-bit export (ING-2/3/4/6)."""

from __future__ import annotations

import logging
from pathlib import Path

import numpy as np
import pytest
import soundfile as sf

from stemwise.audio_io import load_reference, load_references, load_stems, read_audio, write_wav24
from stemwise.errors import AudioFormatError
from stemwise.project import load_project
from tests.helpers.project import StemFixture, make_project, write_audio
from tests.helpers.signals import SR, noise, sine

# Round-trip tolerance for integer PCM: libsndfile scales by 2^(n-1)-1 when writing and by 2^(n-1)
# when reading (up to 1 LSB at full scale), plus float32 rounding near 1.0 (~1 LSB at 24 bit).
LSB = {"PCM_16": 2.0**-15, "PCM_24": 2.0**-23}


def _test_signal(channels: int) -> np.ndarray:
    left = sine(440, amplitude=0.9)
    if channels == 1:
        return left
    return np.hstack([left, noise(amplitude=0.2, seed=3)])


@pytest.mark.req("ING-2", "ING-6")
@pytest.mark.parametrize("channels", [1, 2])
@pytest.mark.parametrize(
    ("ext", "container", "subtype"),
    [
        ("wav", "WAV", "PCM_16"),
        ("wav", "WAV", "PCM_24"),
        ("wav", "WAV", "FLOAT"),
        ("wav", "WAVEX", "PCM_16"),  # WAVE_FORMAT_EXTENSIBLE header (ffmpeg, sox, many DAWs)
        ("wav", "WAVEX", "PCM_24"),
        ("wav", "WAVEX", "FLOAT"),
        ("wav", "RF64", "PCM_24"),
        ("wav", "RF64", "FLOAT"),
        ("flac", "FLAC", "PCM_16"),
        ("flac", "FLAC", "PCM_24"),
    ],
)
def test_read_supported_formats(
    tmp_path: Path, ext: str, container: str, subtype: str, channels: int
) -> None:
    data = _test_signal(channels)
    path = tmp_path / f"a.{ext}"
    sf.write(path, data, SR, subtype=subtype, format=container)
    assert sf.info(str(path)).format == container
    audio, sr = read_audio(path)
    assert sr == SR
    assert audio.dtype == np.float32
    assert audio.shape == (SR, channels)
    atol = 1.5 * LSB[subtype] if subtype in LSB else 0.0  # FLOAT round trip is exact
    np.testing.assert_allclose(audio, data, rtol=0, atol=atol)


@pytest.mark.req("ING-2")
@pytest.mark.parametrize(
    ("subtype", "channels"),
    [("PCM_32", 1), ("DOUBLE", 1), ("PCM_U8", 1), ("PCM_24", 3)],
)
def test_reject_unsupported_format(tmp_path: Path, subtype: str, channels: int) -> None:
    path = write_audio(tmp_path / "bad.wav", np.zeros((100, channels), np.float32), SR, subtype)
    with pytest.raises(AudioFormatError, match=r"bad\.wav"):
        read_audio(path)


@pytest.mark.req("ING-2")
def test_reject_other_containers(tmp_path: Path) -> None:
    path = tmp_path / "a.w64"
    sf.write(path, sine(100), SR, subtype="PCM_24", format="W64")
    with pytest.raises(AudioFormatError, match="W64"):
        read_audio(path)


@pytest.mark.req("ING-4")
def test_all_stems_empty_is_an_error(tmp_path: Path) -> None:
    stems = [StemFixture(i, "other", np.zeros((0, 1), np.float32)) for i in ("a", "b")]
    project = load_project(make_project(tmp_path, stems))
    with pytest.raises(AudioFormatError, match="empty"):
        load_stems(project)


@pytest.mark.req("ING-2")
def test_reject_non_audio_file(tmp_path: Path) -> None:
    path = tmp_path / "notes.wav"
    path.write_text("not audio", encoding="utf-8")
    with pytest.raises(AudioFormatError, match=r"notes\.wav"):
        read_audio(path)


@pytest.mark.req("ING-3")
def test_sample_rate_mismatch_raises(tmp_path: Path) -> None:
    stems = [
        StemFixture("kick", "kick", sine(60)),
        StemFixture("bass", "bass", sine(110, sr=44_100), sr=44_100),
    ]
    project = load_project(make_project(tmp_path, stems))
    with pytest.raises(AudioFormatError) as info:
        load_stems(project)
    message = str(info.value)
    assert "bass" in message
    assert "44100" in message
    assert "48000" in message


@pytest.mark.req("ING-3")
def test_declared_sample_rate_must_match(tmp_path: Path) -> None:
    stems = [StemFixture(i, "other", sine(100, sr=44_100), sr=44_100) for i in ("a", "b")]
    project = load_project(make_project(tmp_path, stems, sample_rate=48_000))
    with pytest.raises(AudioFormatError, match="48000"):
        load_stems(project)


@pytest.mark.req("ING-3")
def test_sample_rate_auto(tmp_path: Path) -> None:
    stems = [StemFixture(i, "other", sine(100, sr=44_100), sr=44_100) for i in ("a", "b")]
    project = load_project(make_project(tmp_path / "ok", stems, sample_rate="auto"))
    assert load_stems(project).sample_rate == 44_100

    mixed = [*stems[:1], StemFixture("c", "other", sine(100))]
    project = load_project(make_project(tmp_path / "mixed", mixed, sample_rate="auto"))
    with pytest.raises(AudioFormatError, match="44100"):
        load_stems(project)


@pytest.mark.req("ING-4")
def test_pad_shorter_stems_with_warning(tmp_path: Path, caplog: pytest.LogCaptureFixture) -> None:
    stems = [
        StemFixture("long", "other", sine(100, duration_s=1.0)),
        StemFixture("short", "other", sine(100, duration_s=0.5, channels=2)),
    ]
    project = load_project(make_project(tmp_path, stems))
    with caplog.at_level(logging.WARNING, logger="stemwise"):
        loaded = load_stems(project)
    assert loaded.frames == SR
    assert loaded.audio["short"].shape == (SR, 2)
    assert np.all(loaded.audio["short"][SR // 2 :] == 0.0)  # exact silence in the tail
    np.testing.assert_allclose(
        loaded.audio["long"], stems[0].data, rtol=0, atol=1.5 * LSB["PCM_24"]
    )
    warnings = [r.getMessage() for r in caplog.records if r.levelno == logging.WARNING]
    assert len(warnings) == 1
    assert "short" in warnings[0]
    assert "long" not in warnings[0].split(":", 1)[1]


@pytest.mark.req("ING-4")
def test_equal_lengths_no_warning(tmp_path: Path, caplog: pytest.LogCaptureFixture) -> None:
    project = load_project(make_project(tmp_path))
    with caplog.at_level(logging.WARNING, logger="stemwise"):
        load_stems(project)
    assert not [r for r in caplog.records if r.levelno >= logging.WARNING]


@pytest.mark.req("ING-6")
def test_internal_dtype_float32(tmp_path: Path) -> None:
    stems = [
        StemFixture("a", "other", sine(100), subtype="PCM_16"),
        StemFixture("b", "other", sine(200, channels=2), subtype="FLOAT", ext="wav"),
        StemFixture("c", "other", sine(300), subtype="PCM_24", ext="flac"),
    ]
    loaded = load_stems(load_project(make_project(tmp_path, stems)))
    assert list(loaded.audio) == ["a", "b", "c"]
    assert {a.dtype for a in loaded.audio.values()} == {np.dtype(np.float32)}
    assert loaded.channels == {"a": 1, "b": 2, "c": 1}


@pytest.mark.req("ING-6")
def test_write_wav_24bit(tmp_path: Path) -> None:
    data = _test_signal(2)
    path = write_wav24(tmp_path / "out.wav", data, SR)
    info = sf.info(path)
    assert (info.format, info.subtype, info.samplerate, info.channels) == ("WAV", "PCM_24", SR, 2)
    back, _ = sf.read(path, dtype="float32", always_2d=True)
    np.testing.assert_allclose(back, data, rtol=0, atol=1.5 * LSB["PCM_24"])


@pytest.mark.req("ING-6")
def test_write_never_overwrites(tmp_path: Path) -> None:
    path = write_wav24(tmp_path / "out.wav", sine(100), SR)
    before = path.read_bytes()
    with pytest.raises(FileExistsError):
        write_wav24(path, sine(200), SR)
    assert path.read_bytes() == before


@pytest.mark.req("ING-5")
def test_reference_resampled_to_project_rate(tmp_path: Path) -> None:
    ref_sr, amplitude = 44_100, 0.5
    path = write_audio(
        tmp_path / "ref.wav", sine(1000, sr=ref_sr, amplitude=amplitude, channels=2), ref_sr
    )
    before = path.read_bytes()

    audio = load_reference(path, SR)

    assert path.read_bytes() == before  # the file on disk is never touched
    assert audio.dtype == np.float32
    assert audio.shape[1] == 2
    assert abs(audio.shape[0] - SR) <= 1  # 1 s at the project rate, ±1 sample of rounding
    spectrum = np.abs(np.fft.rfft(audio[:SR, 0]))
    assert np.argmax(spectrum) == 1000  # 1 Hz bins over 1 s: the tone stays at 1 kHz
    middle = audio[SR // 10 : -SR // 10, 0]  # skip the filter's edge transients
    rms_db = 20 * np.log10(np.sqrt(np.mean(middle.astype(np.float64) ** 2)))
    expected_db = 20 * np.log10(amplitude / np.sqrt(2))
    # ±0.05 dB: 1 kHz is deep in the passband of the anti-aliasing FIR (D-012).
    assert abs(rms_db - expected_db) < 0.05


@pytest.mark.req("ING-5")
def test_reference_at_project_rate_is_not_resampled(tmp_path: Path) -> None:
    data = noise(amplitude=0.1, channels=2, seed=7)
    path = write_audio(tmp_path / "ref.wav", data, SR, "FLOAT")
    np.testing.assert_array_equal(load_reference(path, SR), data)


@pytest.mark.req("ING-5")
def test_load_references_of_project(tmp_path: Path) -> None:
    refs = [("r1.wav", sine(500, sr=44_100), 44_100), ("r2.wav", sine(500, channels=2), SR)]
    project = load_project(make_project(tmp_path, references=refs))
    loaded = load_references(project, SR)
    assert [a.shape[1] for a in loaded] == [1, 2]
    assert all(abs(a.shape[0] - SR) <= 1 for a in loaded)
