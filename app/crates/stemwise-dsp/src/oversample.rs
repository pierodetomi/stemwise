//! 4x oversampler (one channel): two cascaded 2x half-band stages, linear phase, around a nonlinear callback.
//!
//! **Algorithm.** Each stage is a Kaiser-windowed half-band FIR (`h[c] = ½`, zero at the other even offsets, odd
//! taps `½·sinc((k − c)/2)·w[k]` normalised so they sum to ½), in polyphase form: upsampling copies the centre
//! sample and computes one dot product of the odd taps; downsampling computes one dot product of the odd taps over
//! the odd-phase history plus the centre term. Arithmetic in f32 with contiguous, multi-accumulator dot products
//! (vectorisable), histories mirrored to avoid modulo.
//! - Stage 1 (base ↔ 2x): `c = 56` odd taps (113-tap prototype), Kaiser β 8.2: passband to 20 kHz and stopband
//!   from 24.1 kHz at 44.1 kHz (≥ 80 dB).
//! - Stage 2 (2x ↔ 4x): `c = 14` odd taps (29-tap prototype), Kaiser β 7.9: only has to reject the images above
//!   68.2 kHz (≥ 80 dB); what folds into 20–44 kHz at 2x is removed by stage 1 on the way down.
//!
//! **Latency.** Stage 1 delays by `c₁ = 56` base samples round trip, stage 2 by `c₂/2 = 7`; a 1-sample delay line
//! pads the total to exactly [`Oversampler4::LATENCY`] = 64 base samples, a multiple of the engine's 32-sample
//! parameter grid (ENG-5, A-020).
//!
//! **Cost.** About 160 multiply-adds per base sample instead of ~520 for a single 257-tap 4x FIR (A-022).
//!
//! **Properties verified (tests).** Identity round trip flat within 0.1 dB at 1 kHz and 18 kHz (48 kHz), impulse
//! peak exactly at `LATENCY`, stopbands of both stages ≥ 80 dB, reset clears the state.

const C1: usize = 56;
const C2: usize = 14;
const BETA1: f64 = 8.2;
const BETA2: f64 = 7.9;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let q = x * x / 4.0;
    for k in 1..80 {
        term *= q / (k * k) as f64;
        sum += term;
        if term < 1e-18 * sum {
            break;
        }
    }
    sum
}

/// Odd taps `h[c + 1 − 2i]`… as `g[i] = h[2i + 1]` for `i = 0..c` of a half-band prototype with centre `c`.
fn half_band(c: usize, beta: f64) -> Vec<f64> {
    let n = 2 * c + 1;
    let denom = bessel_i0(beta);
    let h: Vec<f64> = (0..n)
        .map(|k| {
            let m = k as f64 - c as f64;
            if k == c {
                return 0.5;
            }
            if (k as isize - c as isize) % 2 == 0 {
                return 0.0;
            }
            let t = m / 2.0;
            let sinc = (std::f64::consts::PI * t).sin() / (std::f64::consts::PI * t);
            let r = m / c as f64;
            0.5 * sinc * bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / denom
        })
        .collect();
    let odd: Vec<f64> = (0..c).map(|i| h[2 * i + 1]).collect();
    let s: f64 = odd.iter().sum();
    odd.iter().map(|v| v * 0.5 / s).collect()
}

#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    let (ca, ra) = a.as_chunks::<8>();
    let (cb, rb) = b.as_chunks::<8>();
    for (x, y) in ca.iter().zip(cb) {
        for k in 0..8 {
            acc[k] += x[k] * y[k];
        }
    }
    let mut s = (acc[0] + acc[4]) + (acc[1] + acc[5]) + (acc[2] + acc[6]) + (acc[3] + acc[7]);
    for (x, y) in ra.iter().zip(rb) {
        s += x * y;
    }
    s
}

/// A mirrored history of the last `n` values, newest first.
#[derive(Debug, Clone)]
struct Hist {
    buf: Vec<f32>,
    pos: usize,
    n: usize,
}

impl Hist {
    fn new(n: usize) -> Self {
        Hist { buf: vec![0.0; 2 * n], pos: 0, n }
    }

    #[inline]
    fn push(&mut self, v: f32) {
        self.pos = if self.pos == 0 { self.n - 1 } else { self.pos - 1 };
        self.buf[self.pos] = v;
        self.buf[self.pos + self.n] = v;
    }

    /// `[x[t], x[t−1], …, x[t−n+1]]`.
    #[inline]
    fn window(&self) -> &[f32] {
        &self.buf[self.pos..self.pos + self.n]
    }

    #[inline]
    fn at(&self, back: usize) -> f32 {
        self.buf[self.pos + back]
    }

    fn clear(&mut self) {
        self.buf.fill(0.0);
        self.pos = 0;
    }
}

/// One 2x half-band stage: an interpolator and a decimator sharing the taps.
#[derive(Debug, Clone)]
struct Stage {
    /// `2·h[2i+1]` for the interpolator.
    up_taps: Vec<f32>,
    /// `h[2i+1]` for the decimator.
    dn_taps: Vec<f32>,
    c: usize,
    up: Hist,
    odd: Hist,
    even: Hist,
}

impl Stage {
    fn new(c: usize, beta: f64) -> Self {
        let g = half_band(c, beta);
        Stage {
            up_taps: g.iter().map(|v| (2.0 * v) as f32).collect(),
            dn_taps: g.iter().map(|v| *v as f32).collect(),
            c,
            up: Hist::new(c),
            odd: Hist::new(c),
            even: Hist::new(c / 2 + 1),
        }
    }

    /// One input sample → two output samples (even, odd), delayed by `c/2` input samples.
    #[inline]
    fn interpolate(&mut self, x: f32) -> (f32, f32) {
        self.up.push(x);
        (self.up.at(self.c / 2), dot(&self.up_taps, self.up.window()))
    }

    /// Two input samples (even, odd) → one output sample, delayed by `c/2` output samples.
    #[inline]
    fn decimate(&mut self, e: f32, o: f32) -> f32 {
        self.even.push(e);
        let y = 0.5 * self.even.at(self.c / 2) + dot(&self.dn_taps, self.odd.window());
        self.odd.push(o);
        y
    }

    fn clear(&mut self) {
        self.up.clear();
        self.odd.clear();
        self.even.clear();
    }
}

/// 4x up/down sampler for one channel.
#[derive(Debug, Clone)]
pub struct Oversampler4 {
    s1: Stage,
    s2: Stage,
    /// One sample of padding so the total latency is 64.
    pad: f32,
}

impl Default for Oversampler4 {
    fn default() -> Self {
        Self::new()
    }
}

impl Oversampler4 {
    /// Round-trip latency (up + down) in base-rate samples.
    pub const LATENCY: usize = C1 + C2 / 2 + 1;

    pub fn new() -> Self {
        Oversampler4 { s1: Stage::new(C1, BETA1), s2: Stage::new(C2, BETA2), pad: 0.0 }
    }

    pub fn latency(&self) -> usize {
        Self::LATENCY
    }

    pub fn reset(&mut self) {
        self.s1.clear();
        self.s2.clear();
        self.pad = 0.0;
    }

    /// Upsamples one base-rate sample into 4, applies `f` to each, downsamples back to one base-rate sample.
    #[inline]
    pub fn process(&mut self, x: f64, mut f: impl FnMut(f64) -> f64) -> f64 {
        f64::from(self.process4(x as f32, |v| {
            for s in v.iter_mut() {
                *s = f(f64::from(*s)) as f32;
            }
        }))
    }

    /// Same, in f32, with the four oversampled points shaped together (`f` gets them in time order): the form the
    /// saturation uses, so the shaper vectorises (A-022).
    #[inline]
    pub fn process4(&mut self, x: f32, f: impl FnOnce(&mut [f32; 4])) -> f32 {
        let (a, b) = self.s1.interpolate(x);
        let (p0, q0) = self.s2.interpolate(a);
        let (p1, q1) = self.s2.interpolate(b);
        let mut v = [p0, q0, p1, q1];
        f(&mut v);
        let d0 = self.s2.decimate(v[0], v[1]);
        let d1 = self.s2.decimate(v[2], v[3]);
        let y = self.s1.decimate(d0, d1);
        let out = std::mem::replace(&mut self.pad, y);
        if out.abs() < 1e-30 { 0.0 } else { out }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone_rms_ratio_db(freq: f64, fs: f64) -> f64 {
        let mut o = Oversampler4::new();
        let n = 9600;
        let w = 2.0 * std::f64::consts::PI * freq / fs;
        let (mut pi, mut po) = (0.0, 0.0);
        for t in 0..n {
            let x = 0.5 * (w * t as f64).sin();
            let y = o.process(x, |v| v);
            if t >= 1000 {
                pi += x * x;
                po += y * y;
            }
        }
        10.0 * (po / pi).log10()
    }

    /// req: FX-4, ENG-5
    #[test]
    fn impulse_peak_at_latency() {
        let mut o = Oversampler4::new();
        let mut out = Vec::new();
        for t in 0..200 {
            out.push(o.process(if t == 0 { 1.0 } else { 0.0 }, |v| v));
        }
        let peak = out.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())).unwrap().0;
        assert_eq!(peak, Oversampler4::LATENCY);
        assert_eq!(Oversampler4::LATENCY, 64);
        // A band-limited impulse peaks a little under 1 (the passband covers ~85% of the band).
        assert!(out[64] > 0.9, "{}", out[64]);
    }

    /// req: FX-4
    #[test]
    fn passband_flat() {
        // Tolerance 0.1 dB: FX-4 passband ripple budget; the 80 dB half-band ripple is far below it.
        for f in [1000.0, 18000.0] {
            let d = tone_rms_ratio_db(f, 48000.0);
            assert!(d.abs() < 0.1, "{f} Hz: {d} dB");
        }
    }

    fn worst_db(c: usize, beta: f64, from: f64) -> f64 {
        let g = half_band(c, beta);
        let n = 2 * c + 1;
        let mut h = vec![0.0; n];
        h[c] = 0.5;
        for (i, v) in g.iter().enumerate() {
            h[2 * i + 1] = *v;
        }
        let mut worst: f64 = -300.0;
        let mut f = from;
        while f <= 0.5 {
            let (mut re, mut im) = (0.0, 0.0);
            for (k, v) in h.iter().enumerate() {
                let ph = 2.0 * std::f64::consts::PI * f * k as f64;
                re += v * ph.cos();
                im -= v * ph.sin();
            }
            worst = worst.max(20.0 * (re * re + im * im).sqrt().log10());
            f += 0.0005;
        }
        worst
    }

    /// req: FX-4
    #[test]
    fn stopbands_at_least_80_db() {
        // Frequencies in cycles per sample of each stage's output rate, for the hardest case (44.1 kHz).
        let s1 = worst_db(C1, BETA1, 24100.0 / 88200.0);
        let s2 = worst_db(C2, BETA2, 68200.0 / 176400.0);
        assert!(s1 < -80.0, "stage 1: {s1} dB");
        assert!(s2 < -80.0, "stage 2: {s2} dB");
    }

    /// req: FX-4
    #[test]
    fn reset_clears_state() {
        let mut o = Oversampler4::new();
        for t in 0..300 {
            o.process((t as f64).sin(), |v| v);
        }
        o.reset();
        for _ in 0..100 {
            assert_eq!(o.process(0.0, |v| v), 0.0);
        }
    }
}
