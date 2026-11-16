//! True peak with 4x oversampling (ANA-1, MAS-3): the PoC's interpolator (poc/docs/analysis.md "True peak"):
//! 81-tap Kaiser (β = 5) windowed sinc, cut-off at the original Nyquist, gain 4, zero-phase, silence outside.

use std::sync::OnceLock;

pub const OVERSAMPLING: usize = 4;
const TAPS: usize = 81;
const HALF: usize = 40;

fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let y = x * x / 4.0;
    for k in 1..200 {
        term *= y / (k as f64 * k as f64);
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

/// The interpolation filter `h` (81 taps, DC gain 4).
pub fn filter() -> &'static [f64; TAPS] {
    static H: OnceLock<[f64; TAPS]> = OnceLock::new();
    H.get_or_init(|| {
        let beta = 5.0;
        let mut g = [0.0; TAPS];
        for (k, gk) in g.iter_mut().enumerate() {
            let t = (k as f64 - HALF as f64) / OVERSAMPLING as f64;
            let sinc = if t == 0.0 { 1.0 } else { (std::f64::consts::PI * t).sin() / (std::f64::consts::PI * t) };
            let r = 2.0 * k as f64 / (TAPS as f64 - 1.0) - 1.0;
            let w = bessel_i0(beta * (1.0 - r * r).max(0.0).sqrt()) / bessel_i0(beta);
            *gk = sinc * w;
        }
        let sum: f64 = g.iter().sum();
        g.map(|v| OVERSAMPLING as f64 * v / sum)
    })
}

/// True peak (linear) of one channel of interleaved audio, never below the sample peak. Public so that offline
/// measures can take the channels on separate threads ([`true_peak_db`] is the maximum over them).
pub fn channel_peak(samples: &[f32], channels: usize, c: usize) -> f64 {
    let h = filter();
    // Read in place (NFR-5: no copy of the signal).
    let n = (samples.len() / channels) as isize;
    let x = |i: usize| f64::from(samples[i * channels + c]);
    let mut peak = (0..n as usize).fold(0.0f64, |a, i| a.max(x(i).abs()));
    for j in 0..n {
        for p in 0..OVERSAMPLING {
            // y[4j + p] = Σ_t h[p + 4t] · x[j + 10 − t]
            let mut acc = 0.0;
            let mut t = 0;
            while p + 4 * t < TAPS {
                let i = j + 10 - t as isize;
                if i >= 0 && i < n {
                    acc += h[p + 4 * t] * x(i as usize);
                }
                t += 1;
            }
            peak = peak.max(acc.abs());
        }
    }
    peak
}

/// True peak in dBTP over all channels (−∞ for silence).
pub fn true_peak_db(samples: &[f32], channels: usize) -> f64 {
    let peak = (0..channels).map(|c| channel_peak(samples, channels, c)).fold(0.0, f64::max);
    20.0 * peak.log10()
}

/// Streaming 4x true-peak detector for one channel (meters and the limiter); no allocation.
/// Output for input sample `n` is the largest interpolated magnitude of the span ending 10 samples earlier
/// (the filter's look-ahead), so callers delay their signal by [`TruePeakStream::LATENCY`] to align.
#[derive(Clone)]
pub struct TruePeakStream {
    hist: [f64; 21],
    pos: usize,
}

impl Default for TruePeakStream {
    fn default() -> Self {
        TruePeakStream { hist: [0.0; 21], pos: 0 }
    }
}

impl TruePeakStream {
    pub const LATENCY: usize = 10;

    pub fn reset(&mut self) {
        self.hist = [0.0; 21];
        self.pos = 0;
    }

    /// Pushes a sample and returns the peak of the 4 interpolated points around the sample `LATENCY` back.
    #[inline]
    pub fn push(&mut self, x: f64) -> f64 {
        self.hist[self.pos] = x;
        self.pos = (self.pos + 1) % 21;
        let h = filter();
        let mut peak = 0.0f64;
        for p in 0..OVERSAMPLING {
            let mut acc = 0.0;
            let mut t = 0;
            while p + 4 * t < TAPS {
                // x[j + 10 − t] with j the centre: the newest sample is x[j + 10].
                let idx = (self.pos + 21 - 1 - t) % 21;
                acc += h[p + 4 * t] * self.hist[idx];
                t += 1;
            }
            peak = peak.max(acc.abs());
        }
        peak
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::{fixture, num, render};

    /// req: ANA-1, MAS-3
    #[test]
    fn true_peak_matches_the_poc_oracle() {
        // Tolerance ±0.1 dB (FOUNDATION §5.7); the filter is the same, differences are rounding only.
        let fx = fixture("stem_measures.json");
        for c in fx["cases"].as_array().unwrap() {
            let fs = c["fs"].as_u64().unwrap() as u32;
            let (x, ch) = render(&c["signal"], fs);
            let want = num(&c["expected"]["true_peak_db"]).unwrap();
            let got = true_peak_db(&x, ch);
            if want.is_infinite() {
                assert_eq!(got, want, "{}", c["id"]);
            } else {
                assert!((got - want).abs() < 0.01, "{}: {got} vs {want}", c["id"]);
            }
        }
    }

    /// req: MAS-3
    #[test]
    fn streaming_detector_sees_the_same_peak() {
        let fs = 48000.0;
        let x: Vec<f32> = (0..4800)
            .map(|n| (std::f64::consts::FRAC_PI_4 + 2.0 * std::f64::consts::PI * 12000.0 * n as f64 / fs).sin() as f32)
            .collect();
        let offline = true_peak_db(&x, 1);
        let mut s = TruePeakStream::default();
        let mut peak = 0.0f64;
        for &v in x.iter().chain(std::iter::repeat_n(&0.0, 16)) {
            peak = peak.max(s.push(f64::from(v)));
        }
        assert!((20.0 * peak.log10() - offline).abs() < 1e-9);
        // The abrupt start and end overshoot (poc/docs/analysis.md); the steady part reads 0 dBTP within 0.02 dB.
        let mut s = TruePeakStream::default();
        let mut mid = 0.0f64;
        for (n, &v) in x.iter().enumerate() {
            let p = s.push(f64::from(v));
            if (100..4700).contains(&n) {
                mid = mid.max(p);
            }
        }
        assert!((20.0 * mid.log10()).abs() < 0.02, "{}", 20.0 * mid.log10());
    }
}
