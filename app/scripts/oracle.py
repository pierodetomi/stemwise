"""Generate the numeric oracle of the product from the frozen PoC (FOUNDATION §5.7).

Run from `poc/` through the PoC environment (`make oracle` in `app/` does it):

    uv run python ../app/scripts/oracle.py

Every signal is defined by a closed formula (or a hand-written xorshift32 generator for noise) that the
Rust tests re-implement identically (`stemwise-dsp::testsignal`). The script measures the signals with the
PoC functions and writes JSON only — parameters and expected values, never audio — to app/fixtures/oracle/.
The PoC is imported, never modified.
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import numpy as np
from scipy import signal as sps

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "poc" / "src"))

from stemwise.analysis import masking, mix, reference, spectral, stem  # noqa: E402
from stemwise.render import biquad, pan  # noqa: E402

OUT = ROOT / "app" / "fixtures" / "oracle"


# ---------------------------------------------------------------------------------------------
# Signals. Keep in sync with app/crates/stemwise-dsp/src/testsignal.rs.
# ---------------------------------------------------------------------------------------------


def xorshift32(seed: int, n: int) -> np.ndarray:
    """Uniform values in [-1, 1): x ^= x<<13; x ^= x>>17; x ^= x<<5 (32-bit), value = x/2^31 - 1."""
    out = np.empty(n, dtype=np.float64)
    x = seed & 0xFFFFFFFF
    if x == 0:
        x = 0x9E3779B9
    for i in range(n):
        x ^= (x << 13) & 0xFFFFFFFF
        x ^= x >> 17
        x ^= (x << 5) & 0xFFFFFFFF
        out[i] = x / 2147483648.0 - 1.0
    return out


def mono(spec: dict, fs: int) -> np.ndarray:
    """Float64 mono signal from a spec (see the Rust module for the same definitions)."""
    kind = spec["kind"]
    n = int(round(spec.get("secs", 0.0) * fs))
    t = np.arange(n, dtype=np.float64)
    if kind == "sine":
        return spec["amp"] * np.sin(2.0 * math.pi * spec["freq"] * t / fs + spec.get("phase", 0.0))
    if kind == "silence":
        return np.zeros(n)
    if kind == "dc":
        return np.full(n, float(spec["amp"]))
    if kind == "noise":
        return spec["amp"] * xorshift32(spec["seed"], n)
    if kind == "pink":
        # Paul Kellet's refined pink filter on xorshift32 white noise, scaled by amp * 0.11.
        w = xorshift32(spec["seed"], n)
        b = [0.0] * 7
        y = np.empty(n)
        for i in range(n):
            x = w[i]
            b[0] = 0.99886 * b[0] + x * 0.0555179
            b[1] = 0.99332 * b[1] + x * 0.0750759
            b[2] = 0.96900 * b[2] + x * 0.1538520
            b[3] = 0.86650 * b[3] + x * 0.3104856
            b[4] = 0.55000 * b[4] + x * 0.5329522
            b[5] = -0.7616 * b[5] - x * 0.0168980
            y[i] = (
                (b[0] + b[1] + b[2] + b[3] + b[4] + b[5] + b[6] + x * 0.5362) * 0.11 * spec["amp"]
            )
            b[6] = x * 0.115926
        return y
    if kind == "clicks":
        # One-sample impulses of `amp` every fs/rate samples, starting at `offset` samples.
        x = np.zeros(n)
        period = int(round(fs / spec["rate"]))
        x[int(spec.get("offset", 0)) :: period] = spec["amp"]
        return x
    if kind == "bursts":
        # Sine bursts: `on` seconds of tone every `period` seconds, rectangular gate.
        tone = spec["amp"] * np.sin(2.0 * math.pi * spec["freq"] * t / fs)
        period = int(round(spec["period"] * fs))
        on = int(round(spec["on"] * fs))
        gate = (np.arange(n) % period) < on
        return tone * gate
    if kind == "concat":
        return np.concatenate([mono(p, fs) for p in spec["parts"]])
    if kind == "sum":
        parts = [mono(p, fs) for p in spec["parts"]]
        length = max(len(p) for p in parts)
        acc = np.zeros(length)
        for p in parts:
            acc[: len(p)] += p
        return acc
    if kind == "gain":
        return mono(spec["of"], fs) * 10.0 ** (spec["db"] / 20.0)
    raise ValueError(f"unknown signal kind {kind}")


def render(spec: dict, fs: int) -> np.ndarray:
    """(frames, channels) float32: a mono spec, or {"kind": "stereo", "left": …, "right": …}."""
    if spec["kind"] == "stereo":
        left, right = mono(spec["left"], fs), mono(spec["right"], fs)
        length = max(len(left), len(right))
        out = np.zeros((length, 2))
        out[: len(left), 0] = left
        out[: len(right), 1] = right
        return out.astype(np.float32)
    return mono(spec, fs).astype(np.float32)[:, None]


# ---------------------------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------------------------


def num(v):
    """JSON-safe float: infinities as ±1e9 sentinels (the Rust side maps them back), None kept."""
    if v is None:
        return None
    v = float(v)
    if math.isinf(v):
        return 1e9 if v > 0 else -1e9
    return v


def stem_expected(m: stem.StemMeasures) -> dict:
    return {
        "channels": m.channels,
        "lufs": num(m.lufs),
        "peak_db": num(m.peak_db),
        "true_peak_db": num(m.true_peak_db),
        "rms_db": num(m.rms_db),
        "crest_db": num(m.crest_db),
        "dyn_range_lu": num(m.dyn_range_lu),
        "active_pct": num(m.active_pct),
        "bands_db": None if m.bands_db is None else [num(b) for b in m.bands_db],
        "centroid_hz": num(m.centroid_hz),
        "onsets_per_s": num(m.onsets_per_s),
        "stereo": None
        if m.stereo is None
        else {"correlation": num(m.stereo.correlation), "side_mid_db": num(m.stereo.side_mid_db)},
        "dc_offset_pct": num(m.hygiene.dc_offset_pct),
        "clipped_samples": m.hygiene.clipped_samples,
        "noise_floor_db": num(m.hygiene.noise_floor_db),
    }


def mix_expected(m: mix.MixMeasures) -> dict:
    return {
        "lufs": num(m.lufs),
        "peak_db": num(m.peak_db),
        "true_peak_db": num(m.true_peak_db),
        "rms_db": num(m.rms_db),
        "crest_db": num(m.crest_db),
        "dyn_range_lu": num(m.dyn_range_lu),
        "bands_db": None if m.bands_db is None else [num(b) for b in m.bands_db],
        "stereo": None
        if m.stereo is None
        else {"correlation": num(m.stereo.correlation), "side_mid_db": num(m.stereo.side_mid_db)},
        "low_corr": num(m.low_corr),
    }


def write(name: str, doc: dict) -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / name
    path.write_text(json.dumps(doc, indent=1) + "\n")
    print(f"wrote {path.relative_to(ROOT)} ({len(doc['cases'])} cases)")


SINE_1K = {"kind": "sine", "freq": 1000.0, "amp": 0.5, "secs": 6.0}
NOISE = {"kind": "noise", "seed": 12345, "amp": 0.25, "secs": 6.0}
PINK = {"kind": "pink", "seed": 777, "amp": 1.0, "secs": 8.0}


def stem_cases() -> list[dict]:
    """ANA-1 → ANA-6 on mono and stereo stems."""
    s = []
    s.append(("sine_1k", SINE_1K, 48000))
    s.append(("sine_997_m20", {"kind": "sine", "freq": 997.0, "amp": 0.1, "secs": 6.0}, 48000))
    s.append(
        (
            "sine_fs4_45deg",
            {"kind": "sine", "freq": 12000.0, "amp": 1.0, "phase": math.pi / 4, "secs": 2.0},
            48000,
        )
    )
    s.append(("noise_white", NOISE, 48000))
    s.append(("noise_pink", PINK, 48000))
    s.append(("pink_441", {**PINK, "secs": 6.0}, 44100))
    s.append(("pink_96k", {**PINK, "secs": 4.0}, 96000))
    s.append(
        (
            "tone_then_silence",
            {
                "kind": "concat",
                "parts": [
                    {"kind": "sine", "freq": 440.0, "amp": 0.3, "secs": 5.0},
                    {"kind": "silence", "secs": 5.0},
                ],
            },
            48000,
        )
    )
    s.append(
        (
            "tone_then_hiss",
            {
                "kind": "concat",
                "parts": [
                    {"kind": "sine", "freq": 220.0, "amp": 0.3, "secs": 4.0},
                    {"kind": "noise", "seed": 99, "amp": 0.0005, "secs": 4.0},
                ],
            },
            48000,
        )
    )
    s.append(
        (
            "clicks_4hz",
            {"kind": "clicks", "rate": 4.0, "amp": 0.8, "secs": 10.0, "offset": 2400},
            48000,
        )
    )
    s.append(
        (
            "bursts",
            {"kind": "bursts", "freq": 330.0, "amp": 0.4, "period": 0.5, "on": 0.12, "secs": 8.0},
            48000,
        )
    )
    s.append(
        (
            "two_sines",
            {
                "kind": "sum",
                "parts": [
                    {"kind": "sine", "freq": 500.0, "amp": 0.3, "secs": 5.0},
                    {"kind": "sine", "freq": 2000.0, "amp": 0.3, "secs": 5.0},
                ],
            },
            48000,
        )
    )
    s.append(
        (
            "dc_and_sine",
            {
                "kind": "sum",
                "parts": [
                    {"kind": "dc", "amp": 0.01, "secs": 5.0},
                    {"kind": "sine", "freq": 100.0, "amp": 0.2, "secs": 5.0},
                ],
            },
            48000,
        )
    )
    s.append(("clipped", {"kind": "sine", "freq": 100.0, "amp": 1.2, "secs": 3.0}, 48000))
    s.append(("silence", {"kind": "silence", "secs": 3.0}, 48000))
    s.append(("short", {"kind": "sine", "freq": 1000.0, "amp": 0.5, "secs": 0.3}, 48000))
    s.append(
        (
            "lra_3342_a",
            {
                "kind": "concat",
                "parts": [
                    {"kind": "sine", "freq": 1000.0, "amp": 10 ** (-20 / 20), "secs": 20.0},
                    {"kind": "sine", "freq": 1000.0, "amp": 10 ** (-30 / 20), "secs": 20.0},
                ],
            },
            48000,
        )
    )
    # Stereo stems
    s.append(("stereo_dual_mono", {"kind": "stereo", "left": NOISE, "right": NOISE}, 48000))
    s.append(
        (
            "stereo_antiphase",
            {
                "kind": "stereo",
                "left": SINE_1K,
                "right": {"kind": "gain", "db": 0.0, "of": {**SINE_1K, "phase": math.pi}},
            },
            48000,
        )
    )
    s.append(
        (
            "stereo_hard_left",
            {"kind": "stereo", "left": PINK, "right": {"kind": "silence", "secs": 8.0}},
            48000,
        )
    )
    s.append(
        (
            "stereo_wide",
            {
                "kind": "stereo",
                "left": {"kind": "noise", "seed": 1, "amp": 0.2, "secs": 6.0},
                "right": {
                    "kind": "sum",
                    "parts": [
                        {"kind": "noise", "seed": 1, "amp": 0.1, "secs": 6.0},
                        {"kind": "noise", "seed": 2, "amp": 0.15, "secs": 6.0},
                    ],
                },
            },
            48000,
        )
    )
    s.append(
        (
            "stereo_clicks_antiphase",
            {
                "kind": "stereo",
                "left": {"kind": "clicks", "rate": 4.0, "amp": 0.8, "secs": 10.0, "offset": 2400},
                "right": {"kind": "clicks", "rate": 4.0, "amp": -0.8, "secs": 10.0, "offset": 2400},
            },
            48000,
        )
    )
    cases = []
    for cid, spec, fs in s:
        audio = render(spec, fs)
        m = stem.measure_stem(audio, fs)
        cases.append({"id": cid, "fs": fs, "signal": spec, "expected": stem_expected(m)})
    return cases


def mix_cases() -> list[dict]:
    """ANA-7 and ANA-10 on stereo mixes."""
    s = [
        (
            "mix_pink_wide",
            {
                "kind": "stereo",
                "left": PINK,
                "right": {
                    "kind": "sum",
                    "parts": [
                        {"kind": "gain", "db": -6.0, "of": PINK},
                        {"kind": "pink", "seed": 778, "amp": 0.6, "secs": 8.0},
                    ],
                },
            },
            48000,
        ),
        (
            "mix_bass_antiphase",
            {
                "kind": "stereo",
                "left": {"kind": "sine", "freq": 60.0, "amp": 0.4, "secs": 6.0},
                "right": {"kind": "sine", "freq": 60.0, "amp": 0.4, "phase": math.pi, "secs": 6.0},
            },
            48000,
        ),
        ("mix_mono_noise", {"kind": "stereo", "left": NOISE, "right": NOISE}, 44100),
    ]
    cases = []
    for cid, spec, fs in s:
        audio = render(spec, fs)
        cases.append(
            {
                "id": cid,
                "fs": fs,
                "signal": spec,
                "expected": mix_expected(mix.measure_mix(audio, fs)),
            }
        )
    return cases


def masking_cases() -> list[dict]:
    """ANA-8 band shares and ANA-9 pair scores."""
    fs = 48000
    sets = [
        ("identical", {"a": PINK, "b": PINK}),
        ("copy_minus_12", {"a": PINK, "b": {"kind": "gain", "db": -12.0, "of": PINK}}),
        (
            "disjoint_bands",
            {
                "a": {"kind": "sine", "freq": 80.0, "amp": 0.3, "secs": 6.0},
                "b": {"kind": "sine", "freq": 3000.0, "amp": 0.3, "secs": 6.0},
            },
        ),
        (
            "alternating",
            {
                "a": {
                    "kind": "bursts",
                    "freq": 440.0,
                    "amp": 0.3,
                    "period": 1.0,
                    "on": 0.5,
                    "secs": 6.0,
                },
                "b": {
                    "kind": "concat",
                    "parts": [
                        {"kind": "silence", "secs": 0.5},
                        {
                            "kind": "bursts",
                            "freq": 440.0,
                            "amp": 0.3,
                            "period": 1.0,
                            "on": 0.5,
                            "secs": 5.5,
                        },
                    ],
                },
            },
        ),
        (
            "stereo_and_mono",
            {
                "a": {
                    "kind": "stereo",
                    "left": NOISE,
                    "right": {"kind": "noise", "seed": 5, "amp": 0.25, "secs": 6.0},
                },
                "b": {"kind": "gain", "db": -6.0, "of": NOISE},
            },
        ),
    ]
    cases = []
    for cid, stems in sets:
        specs = {k: spectral.analyze_spectrum(render(v, fs), fs) for k, v in stems.items()}
        cases.append(
            {
                "id": cid,
                "fs": fs,
                "signals": stems,
                "expected": {
                    "pair_scores": [num(v) for v in masking.pair_scores(specs["a"], specs["b"])],
                    "band_share_pct": {
                        k: [num(x) for x in v] for k, v in mix.band_share_pct(specs).items()
                    },
                },
            }
        )
    return cases


def reference_cases() -> list[dict]:
    """ANA-11 normalised reference measures and the ANA-12 target profile."""
    fs = 48000
    refs = [
        {
            "kind": "stereo",
            "left": PINK,
            "right": {
                "kind": "sum",
                "parts": [
                    {"kind": "gain", "db": -3.0, "of": PINK},
                    {"kind": "noise", "seed": 31, "amp": 0.05, "secs": 8.0},
                ],
            },
        },
        {
            "kind": "stereo",
            "left": {"kind": "gain", "db": -12.0, "of": NOISE},
            "right": {
                "kind": "gain",
                "db": -12.0,
                "of": {"kind": "noise", "seed": 32, "amp": 0.25, "secs": 6.0},
            },
        },
        {"kind": "sine", "freq": 200.0, "amp": 0.5, "secs": 6.0},
    ]
    measured = [reference.measure_reference(render(r, fs), fs) for r in refs]
    t = reference.target_profile(measured)
    return [
        {
            "id": "three_refs",
            "fs": fs,
            "signals": refs,
            "expected": {
                "references": [mix_expected(m) for m in measured],
                "target": {
                    "n_references": t.n_references,
                    "bands_db": None if t.bands_db is None else [num(b) for b in t.bands_db],
                    "side_mid_db": num(t.side_mid_db),
                    "dyn_range_lu": num(t.dyn_range_lu),
                },
            },
        }
    ]


def biquad_cases() -> list[dict]:
    """FX-2: frequency responses of the RBJ biquads (dB) at fixed probe frequencies."""
    probes = [
        20,
        31.5,
        50,
        80,
        100,
        160,
        250,
        400,
        630,
        1000,
        1600,
        2500,
        4000,
        6300,
        10000,
        16000,
        20000,
    ]
    filters = []
    for fs in (44100, 48000, 96000):
        filters += [
            ("hpf", 90.0, 0.0, biquad.BUTTERWORTH_Q, fs),
            ("hpf", 400.0, 0.0, biquad.BUTTERWORTH_Q, fs),
            ("lpf", 2000.0, 0.0, biquad.BUTTERWORTH_Q, fs),
            ("lpf", 18000.0, 0.0, biquad.BUTTERWORTH_Q, fs),
            ("peak", 300.0, -2.5, 1.4, fs),
            ("peak", 3000.0, 9.0, 8.0, fs),
            ("peak", 50.0, -9.0, 0.3, fs),
            ("low_shelf", 120.0, 4.0, 0.7, fs),
            ("low_shelf", 400.0, -6.0, 1.0, fs),
            ("high_shelf", 9000.0, 1.5, 0.7, fs),
            ("high_shelf", 4000.0, -9.0, 0.5, fs),
        ]
    cases = []
    for kind, f0, g, q, fs in filters:
        if kind == "hpf":
            sos = biquad.highpass(f0, fs, q)
        elif kind == "lpf":
            sos = biquad.lowpass(f0, fs, q)
        else:
            sos = biquad.eq_band(kind, f0, g, q, fs)
        freqs = [f for f in probes if f < fs / 2]
        _, h = sps.sosfreqz(sos, worN=np.array(freqs, dtype=np.float64), fs=fs)
        cases.append(
            {
                "id": f"{kind}_{f0:g}_{g:g}_{q:g}_{fs}",
                "kind": kind,
                "freq_hz": f0,
                "gain_db": g,
                "q": q,
                "fs": fs,
                "coefficients": [float(c) for c in sos[0]],
                "response": [
                    {"freq_hz": f, "db": float(20 * np.log10(abs(v)))} for f, v in zip(freqs, h)
                ],
            }
        )
    return cases


def pan_cases() -> list[dict]:
    """REN-3 / TRK-2 pan laws."""
    cases = []
    for p in (-1.0, -0.75, -0.5, -0.25, 0.0, 0.3, 0.5, 1.0):
        cases.append(
            {
                "id": f"pan_{p:g}",
                "pan": p,
                "mono_gains": list(pan.mono_pan_gains(p)),
                "balance_gains": list(pan.balance_gains(p)),
            }
        )
    return cases


def main() -> None:
    write(
        "stem_measures.json",
        {"version": 1, "source": "PoC stemwise.analysis.stem.measure_stem", "cases": stem_cases()},
    )
    write(
        "mix_measures.json",
        {"version": 1, "source": "PoC stemwise.analysis.mix.measure_mix", "cases": mix_cases()},
    )
    write(
        "masking.json",
        {
            "version": 1,
            "source": "PoC stemwise.analysis.masking / mix.band_share_pct",
            "cases": masking_cases(),
        },
    )
    write(
        "reference.json",
        {"version": 1, "source": "PoC stemwise.analysis.reference", "cases": reference_cases()},
    )
    write(
        "biquad.json",
        {
            "version": 1,
            "source": "PoC stemwise.render.biquad + scipy.signal.sosfreqz",
            "cases": biquad_cases(),
        },
    )
    write("pan.json", {"version": 1, "source": "PoC stemwise.render.pan", "cases": pan_cases()})


if __name__ == "__main__":
    main()
