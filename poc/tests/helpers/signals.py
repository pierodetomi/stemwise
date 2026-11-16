"""Synthetic test signals (PRD §10.1). All return float32 arrays shaped (samples, channels)."""

from __future__ import annotations

import numpy as np

SR = 48_000


def _frames(sr: int, duration_s: float) -> int:
    return round(sr * duration_s)


def sine(
    freq_hz: float,
    *,
    sr: int = SR,
    duration_s: float = 1.0,
    amplitude: float = 0.5,
    phase: float = 0.0,
    channels: int = 1,
) -> np.ndarray:
    t = np.arange(_frames(sr, duration_s)) / sr
    x = amplitude * np.sin(2 * np.pi * freq_hz * t + phase)
    return np.repeat(x[:, None], channels, axis=1).astype(np.float32)


def silence(*, sr: int = SR, duration_s: float = 1.0, channels: int = 1) -> np.ndarray:
    return np.zeros((_frames(sr, duration_s), channels), dtype=np.float32)


def noise(
    *,
    sr: int = SR,
    duration_s: float = 1.0,
    amplitude: float = 0.1,
    channels: int = 1,
    seed: int = 0,
) -> np.ndarray:
    """Gaussian white noise with standard deviation `amplitude`, independent per channel."""
    rng = np.random.default_rng(seed)
    return (amplitude * rng.standard_normal((_frames(sr, duration_s), channels))).astype(np.float32)


def fade(x: np.ndarray, *, sr: int = SR, ms: float = 10.0) -> np.ndarray:
    """Raised-cosine fade-in and fade-out, to avoid edge overshoot from abrupt starts/stops."""
    n = round(sr * ms / 1000)
    ramp = (0.5 - 0.5 * np.cos(np.pi * np.arange(n) / n)).astype(np.float32)[:, None]
    y = x.copy()
    y[:n] *= ramp
    y[-n:] *= ramp[::-1]
    return y


def pink_noise(
    *,
    sr: int = SR,
    duration_s: float = 30.0,
    rms: float = 0.1,
    channels: int = 1,
    seed: int = 0,
) -> np.ndarray:
    """Pink noise with an exact 1/f power spectrum: |X(f)| ∝ f^(-1/2), DC = 0, random phase.

    Built in the frequency domain over the whole length (so the power per octave is exactly
    constant), then scaled to the given RMS. Channels have independent phases.
    """
    n = _frames(sr, duration_s)
    rng = np.random.default_rng(seed)
    freqs = np.fft.rfftfreq(n, d=1.0 / sr)
    magnitude = np.zeros_like(freqs)
    magnitude[1:] = freqs[1:] ** -0.5
    out = np.empty((n, channels))
    for c in range(channels):
        phase = rng.uniform(0.0, 2.0 * np.pi, size=freqs.shape)
        x = np.fft.irfft(magnitude * np.exp(1j * phase), n=n)
        out[:, c] = x * (rms / np.sqrt(np.mean(x**2)))
    return out.astype(np.float32)


def clicks(
    rate_hz: float,
    *,
    sr: int = SR,
    duration_s: float = 10.0,
    amplitude: float = 0.5,
    start_s: float = 0.125,
) -> np.ndarray:
    """Single-sample impulses every 1 / rate_hz seconds from start_s, mono."""
    x = np.zeros((_frames(sr, duration_s), 1), dtype=np.float32)
    step = sr / rate_hz
    k = 0
    while (i := round(sr * start_s + k * step)) < x.shape[0]:
        x[i, 0] = amplitude
        k += 1
    return x


def noise_bursts(
    rate_hz: float,
    *,
    sr: int = SR,
    duration_s: float = 10.0,
    amplitude: float = 0.5,
    decay_ms: float = 15.0,
    length_ms: float = 80.0,
    start_s: float = 0.125,
    seed: int = 0,
) -> np.ndarray:
    """Exponentially decaying white-noise bursts (drum-like hits) every 1 / rate_hz s, mono."""
    rng = np.random.default_rng(seed)
    x = np.zeros((_frames(sr, duration_s), 1))
    n = round(sr * length_ms / 1000)
    env = np.exp(-np.arange(n) / (sr * decay_ms / 1000))
    k = 0
    while (i := round(sr * start_s + k * sr / rate_hz)) + n <= x.shape[0]:
        x[i : i + n, 0] += amplitude * env * rng.standard_normal(n)
        k += 1
    return x.astype(np.float32)
