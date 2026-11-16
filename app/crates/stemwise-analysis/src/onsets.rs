//! Transient density (ANA-4, `onsets v2`, D-020/D-023): port of the PoC's `onset_samples` / `onsets_per_s`, which
//! call librosa 1.0.0.
//!
//! Algorithm, per channel: centred STFT (n_fft 2048, hop 512, periodic Hann, zero padding, `1 + n / hop` frames);
//! power spectrogram; 128-band Slaney mel filterbank (`htk=False`, `norm="slaney"`, 0 Hz to rate/2), computed once
//! per call and applied sparsely; `power_to_db` (ref 1, amin 1e-10, top_db 80 below the spectrogram maximum);
//! positive frame difference (lag 1) averaged over the bands and shifted by `lag + n_fft / (2 hop)` = 3 frames
//! (`center=True`), trimmed to the frame count. The per-channel envelopes are averaged, then librosa's greedy
//! `peak_pick` runs with `delta = 2.0` and the default 30 ms / 0 ms / 100 ms / 100 ms / 30 ms windows derived from
//! the rate and the hop (`normalize=False`). Frames convert to samples as `frame * hop` (no centring offset, since
//! librosa's `onset_detect` passes no `n_fft` to `frames_to_samples`).
//!
//! Parity: the arithmetic is float64 where librosa uses float32 for the STFT, the mel matrix and the dB
//! spectrogram; the oracle fixture `stem_measures.json` is compared with a tolerance of 0.05 onset/s (see the
//! test for the cases and the observed deviation).

use crate::spectral::{Spectrum, active_seconds};
use realfft::RealFftPlanner;

const N_FFT: usize = 2048;
const HOP: usize = 512;
const N_MELS: usize = 128;
const DELTA: f64 = 2.0;
const AMIN: f64 = 1e-10;
const TOP_DB: f64 = 80.0;

fn hz_to_mel(f: f64) -> f64 {
    let f_sp = 200.0 / 3.0;
    let min_log_mel = 1000.0 / f_sp;
    let logstep = 6.4f64.ln() / 27.0;
    if f >= 1000.0 { min_log_mel + (f / 1000.0).ln() / logstep } else { f / f_sp }
}

fn mel_to_hz(m: f64) -> f64 {
    let f_sp = 200.0 / 3.0;
    let min_log_mel = 1000.0 / f_sp;
    let logstep = 6.4f64.ln() / 27.0;
    if m >= min_log_mel { 1000.0 * (logstep * (m - min_log_mel)).exp() } else { f_sp * m }
}

/// Sparse Slaney mel filterbank: per band, the first bin and the weights of its non-zero run.
fn mel_filterbank(rate: u32) -> Vec<(usize, Vec<f64>)> {
    let fmax = f64::from(rate) / 2.0;
    let (lo, hi) = (hz_to_mel(0.0), hz_to_mel(fmax));
    let mel_f: Vec<f64> = (0..N_MELS + 2).map(|i| mel_to_hz(lo + (hi - lo) * i as f64 / (N_MELS + 1) as f64)).collect();
    let bins = N_FFT / 2 + 1;
    let freqs: Vec<f64> = (0..bins).map(|k| f64::from(rate) * k as f64 / N_FFT as f64).collect();
    (0..N_MELS)
        .map(|i| {
            let fd0 = mel_f[i + 1] - mel_f[i];
            let fd1 = mel_f[i + 2] - mel_f[i + 1];
            let enorm = 2.0 / (mel_f[i + 2] - mel_f[i]);
            let w: Vec<f64> = freqs
                .iter()
                .map(|&f| {
                    let lower = (f - mel_f[i]) / fd0;
                    let upper = (mel_f[i + 2] - f) / fd1;
                    lower.min(upper).max(0.0) * enorm
                })
                .collect();
            let first = w.iter().position(|&x| x > 0.0).unwrap_or(0);
            let last = w.iter().rposition(|&x| x > 0.0).map_or(first, |p| p + 1);
            (first, w[first..last.max(first)].to_vec())
        })
        .collect()
}

/// Onset strength envelope of one channel (librosa `onset_strength`, defaults).
fn channel_envelope(x: &[f32], channels: usize, ch: usize, n: usize, mel: &[(usize, Vec<f64>)]) -> Vec<f64> {
    let frames = 1 + n / HOP;
    let bins = N_FFT / 2 + 1;
    let window: Vec<f64> =
        (0..N_FFT).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / N_FFT as f64).cos()).collect();
    let mut planner = RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(N_FFT);
    let mut input = fft.make_input_vec();
    let mut output = fft.make_output_vec();
    let mut scratch = fft.make_scratch_vec();
    let mut power = vec![0.0f64; bins];
    // dB mel spectrogram, frame-major.
    let mut db = vec![0.0f64; frames * N_MELS];
    let sample = |idx: isize| -> f64 {
        if idx < 0 || idx as usize >= n { 0.0 } else { f64::from(x[idx as usize * channels + ch]) }
    };
    for t in 0..frames {
        let start = (t * HOP) as isize - (N_FFT / 2) as isize;
        for (i, v) in input.iter_mut().enumerate() {
            *v = sample(start + i as isize) * window[i];
        }
        fft.process_with_scratch(&mut input, &mut output, &mut scratch).expect("fft size");
        for (p, c) in power.iter_mut().zip(output.iter()) {
            *p = c.re * c.re + c.im * c.im;
        }
        for (m, (first, w)) in mel.iter().enumerate() {
            let s: f64 = w.iter().zip(&power[*first..]).map(|(a, b)| a * b).sum();
            db[t * N_MELS + m] = 10.0 * s.max(AMIN).log10();
        }
    }
    let max = db.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let floor = max - TOP_DB;
    for v in &mut db {
        *v = v.max(floor);
    }
    // Positive lag-1 difference, mean over bands, shifted by lag + n_fft / (2 hop) frames and trimmed.
    let shift = 1 + N_FFT / (2 * HOP);
    let mut env = vec![0.0f64; frames];
    for (t, e) in env.iter_mut().enumerate() {
        if t < shift {
            continue;
        }
        let i = t - shift;
        let s: f64 = (0..N_MELS).map(|m| (db[(i + 1) * N_MELS + m] - db[i * N_MELS + m]).max(0.0)).sum();
        *e = s / N_MELS as f64;
    }
    env
}

/// Python's `a // b` on floats, as librosa uses for its default window lengths.
fn floor_div(a: f64, b: f64) -> f64 {
    ((a - a % b) / b).round()
}

/// librosa `peak_pick` (greedy), sparse output.
fn peak_pick(x: &[f64], pre_max: usize, post_max: usize, pre_avg: usize, post_avg: usize, wait: usize) -> Vec<usize> {
    let len = x.len();
    let max_of = |a: usize, b: usize| x[a..b].iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean_of = |a: usize, b: usize| x[a..b].iter().sum::<f64>() / (b - a) as f64;
    let mut peaks = Vec::new();
    let mut n = 1;
    if len > 0 && x[0] >= max_of(0, post_max.min(len)) && x[0] >= mean_of(0, post_avg.min(len)) + DELTA {
        peaks.push(0);
        n = wait + 1;
    }
    while n < len {
        let maxn = max_of(n.saturating_sub(pre_max), (n + post_max).min(len));
        if x[n] == maxn && x[n] >= mean_of(n.saturating_sub(pre_avg), (n + post_avg).min(len)) + DELTA {
            peaks.push(n);
            n += wait + 1;
        } else {
            n += 1;
        }
    }
    peaks
}

/// Onset positions in samples, librosa-compatible (mean of the per-channel onset envelopes).
pub fn onset_samples(samples: &[f32], channels: usize, rate: u32) -> Vec<usize> {
    if channels == 0 {
        return Vec::new();
    }
    let n = samples.len() / channels;
    if n == 0 {
        return Vec::new();
    }
    let mel = mel_filterbank(rate);
    let frames = 1 + n / HOP;
    let mut env = vec![0.0f64; frames];
    for ch in 0..channels {
        for (e, v) in env.iter_mut().zip(channel_envelope(samples, channels, ch, n, &mel)) {
            *e += v;
        }
    }
    for e in &mut env {
        *e /= channels as f64;
    }
    if !env.iter().any(|&v| v != 0.0) || env.iter().any(|v| !v.is_finite()) {
        return Vec::new();
    }
    let sr = f64::from(rate);
    let h = HOP as f64;
    let pre_max = floor_div(0.03 * sr, h) as usize;
    let post_max = (floor_div(0.00 * sr, h) + 1.0) as usize;
    let pre_avg = floor_div(0.10 * sr, h) as usize;
    let post_avg = (floor_div(0.10 * sr, h) + 1.0) as usize;
    let wait = floor_div(0.03 * sr, h) as usize;
    peak_pick(&env, pre_max, post_max, pre_avg, post_avg, wait).into_iter().map(|f| f * HOP).collect()
}

/// ANA-4: onsets falling in active analysis frames, per second of active time (D-023); 0 with no active frames.
pub fn onsets_per_s(samples: &[f32], channels: usize, rate: u32, spec: &Spectrum) -> f64 {
    let seconds = active_seconds(spec);
    if channels == 0 || seconds <= 0.0 || samples.len() / channels < 2 || spec.n_frames() == 0 {
        return 0.0;
    }
    let last = spec.n_frames() - 1;
    let count = onset_samples(samples, channels, rate)
        .into_iter()
        .filter(|&s| {
            let t = ((s as f64 - spec.n_fft as f64 / 2.0) / spec.hop as f64).round_ties_even();
            spec.active[t.clamp(0.0, last as f64) as usize]
        })
        .count();
    count as f64 / seconds
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spectral::analyze;
    use serde_json::json;
    use stemwise_dsp::testsignal::{fixture, num, render};

    fn per_s(spec: &serde_json::Value, fs: u32) -> f64 {
        let (x, ch) = render(spec, fs);
        onsets_per_s(&x, ch, fs, &analyze(&x, ch, fs))
    }

    /// req: ANA-4
    /// Parity with the PoC oracle. Tolerance 0.05 onset/s: one onset more or less on a 6-20 s signal moves the
    /// value by 0.05-0.17, so any peak decision that differs fails; float64 vs float32 only moves values by
    /// rounding noise unless a peak sits exactly on the threshold.
    #[test]
    fn matches_the_oracle() {
        let fx = fixture("stem_measures.json");
        let mut failures = Vec::new();
        for case in fx["cases"].as_array().unwrap() {
            let fs = case["fs"].as_u64().unwrap() as u32;
            let want = num(&case["expected"]["onsets_per_s"]).unwrap();
            let got = per_s(&case["signal"], fs);
            if (got - want).abs() > 0.05 {
                failures.push(format!("{}: got {got}, want {want}", case["id"]));
            }
        }
        assert!(failures.is_empty(), "{failures:?}");
    }

    /// req: ANA-4
    /// 4 clicks/s over 10 s give exactly 40 onsets, i.e. 4.0 per second of signal, mono and anti-phase stereo (the
    /// envelopes are averaged, not the channels). Exact count: every click is a clear peak. The density per active
    /// second is higher (the oracle gives 8.67) because only the frames around the clicks are active.
    #[test]
    fn clicks_give_four_per_second() {
        let click = json!({"kind": "clicks", "rate": 4.0, "amp": 0.8, "secs": 10.0, "offset": 2400});
        let anti = json!({"kind": "stereo", "left": click, "right": {"kind": "clicks", "rate": 4.0, "amp": -0.8, "secs": 10.0, "offset": 2400}});
        for sig in [click, anti] {
            let (x, ch) = render(&sig, 48000);
            let n = onset_samples(&x, ch, 48000).len();
            assert_eq!(n, 40);
            assert!(onsets_per_s(&x, ch, 48000, &analyze(&x, ch, 48000)) > 4.0);
        }
    }

    /// req: ANA-4
    /// Stationary signals have no transients: at most 0.1 onset/s.
    #[test]
    fn stationary_signals_have_no_onsets() {
        let sine = json!({"kind": "sine", "freq": 440.0, "amp": 0.5, "secs": 10.0});
        assert!(per_s(&sine, 48000) <= 0.1);
        let n = 48000 * 10;
        let white: Vec<f32> =
            stemwise_dsp::testsignal::xorshift32(7, n).into_iter().map(|v| (0.3 * v) as f32).collect();
        assert!(onsets_per_s(&white, 1, 48000, &analyze(&white, 1, 48000)) <= 0.1);
        // Pink-ish noise: one-pole low-passed white noise.
        let mut acc = 0.0f64;
        let pink: Vec<f32> = stemwise_dsp::testsignal::xorshift32(11, n)
            .into_iter()
            .map(|v| {
                acc = 0.98 * acc + 0.02 * v;
                (6.0 * acc) as f32
            })
            .collect();
        assert!(onsets_per_s(&pink, 1, 48000, &analyze(&pink, 1, 48000)) <= 0.1);
    }

    /// req: ANA-4
    /// Silence and empty input give no onsets and a zero density.
    #[test]
    fn silence_is_zero() {
        assert!(onset_samples(&[], 1, 48000).is_empty());
        let z = vec![0f32; 48000];
        assert!(onset_samples(&z, 1, 48000).is_empty());
        assert_eq!(onsets_per_s(&z, 1, 48000, &analyze(&z, 1, 48000)), 0.0);
    }
}
