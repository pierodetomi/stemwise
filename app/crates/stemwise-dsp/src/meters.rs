//! Meters (TRK-2, MAS-4): peak and RMS per channel, stereo correlation, and a spectrum analyser.
//!
//! The audio thread only updates the cheap accumulators (`PeakRms`, `Correlation`); the spectrum is computed off
//! the audio thread from samples copied into a lock-free ring (ENG-6), so its FFT never runs in the callback.

use std::sync::Arc;

use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};

/// Peak and RMS of one channel since the last `take`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PeakRms {
    peak: f32,
    sum_sq: f64,
    n: u32,
}

impl PeakRms {
    #[inline]
    pub fn feed(&mut self, buf: &[f32]) {
        for &s in buf {
            let a = s.abs();
            if a > self.peak {
                self.peak = a;
            }
            self.sum_sq += f64::from(s) * f64::from(s);
        }
        self.n += buf.len() as u32;
    }

    /// (peak dBFS, RMS dBFS) since the last call; −∞ for silence.
    pub fn take(&mut self) -> (f64, f64) {
        let peak = 20.0 * f64::from(self.peak).log10();
        let rms = if self.n == 0 { f64::NEG_INFINITY } else { 10.0 * (self.sum_sq / f64::from(self.n)).log10() };
        *self = PeakRms::default();
        (peak, rms)
    }
}

/// Zero-lag correlation of L and R since the last `take`, in −1…1 (1 for silence, as meters show it).
#[derive(Debug, Clone, Copy, Default)]
pub struct Correlation {
    ll: f64,
    rr: f64,
    lr: f64,
}

impl Correlation {
    #[inline]
    pub fn feed(&mut self, l: &[f32], r: &[f32]) {
        for (a, b) in l.iter().zip(r) {
            let (a, b) = (f64::from(*a), f64::from(*b));
            self.ll += a * a;
            self.rr += b * b;
            self.lr += a * b;
        }
    }

    pub fn take(&mut self) -> f64 {
        let c = if self.ll <= 0.0 || self.rr <= 0.0 {
            1.0
        } else {
            (self.lr / (self.ll * self.rr).sqrt()).clamp(-1.0, 1.0)
        };
        *self = Correlation::default();
        c
    }
}

/// Spectrum analyser: Hann-windowed FFT of 4096 points, levels in dB per bin, smoothed over time.
pub struct Spectrum {
    fft: Arc<dyn RealToComplex<f64>>,
    window: Vec<f64>,
    input: Vec<f64>,
    output: Vec<Complex<f64>>,
    scratch: Vec<Complex<f64>>,
    smoothed: Vec<f64>,
    pub size: usize,
}

impl Spectrum {
    pub fn new(size: usize) -> Self {
        let mut planner = RealFftPlanner::<f64>::new();
        let fft = planner.plan_fft_forward(size);
        let window: Vec<f64> =
            (0..size).map(|n| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * n as f64 / size as f64).cos()).collect();
        Spectrum {
            input: fft.make_input_vec(),
            output: fft.make_output_vec(),
            scratch: fft.make_scratch_vec(),
            fft,
            window,
            smoothed: vec![-120.0; size / 2 + 1],
            size,
        }
    }

    /// Analyses the last `size` mono samples; returns the smoothed level of each bin (dBFS for a full-scale sine).
    pub fn analyse(&mut self, samples: &[f32], smoothing: f64) -> &[f64] {
        let n = self.size.min(samples.len());
        let wsum: f64 = self.window.iter().sum();
        for i in 0..self.size {
            self.input[i] = if i < n { f64::from(samples[samples.len() - n + i]) * self.window[i] } else { 0.0 };
        }
        if self.fft.process_with_scratch(&mut self.input, &mut self.output, &mut self.scratch).is_ok() {
            for (k, c) in self.output.iter().enumerate() {
                let mag = 2.0 * c.norm() / wsum;
                let db = (20.0 * mag.max(1e-9).log10()).max(-120.0);
                self.smoothed[k] = smoothing * self.smoothed[k] + (1.0 - smoothing) * db;
            }
        }
        &self.smoothed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// req: TRK-2
    #[test]
    fn peak_rms_and_correlation() {
        let s: Vec<f32> = (0..48000).map(|n| (0.5 * (n as f64 * 0.1).sin()) as f32).collect();
        let mut m = PeakRms::default();
        m.feed(&s);
        let (p, r) = m.take();
        assert!((p - 20.0 * 0.5f64.log10()).abs() < 0.01);
        assert!((r - 20.0 * (0.5 / 2f64.sqrt()).log10()).abs() < 0.01);
        assert_eq!(m.take().0, f64::NEG_INFINITY);
        let mut c = Correlation::default();
        let inv: Vec<f32> = s.iter().map(|v| -v).collect();
        c.feed(&s, &s);
        assert!((c.take() - 1.0).abs() < 1e-12);
        c.feed(&s, &inv);
        assert!((c.take() + 1.0).abs() < 1e-12);
    }

    /// req: MAS-4, FX-7
    #[test]
    fn spectrum_finds_a_sine_at_its_level() {
        let rate = 48000.0;
        let f = 1500.0;
        let s: Vec<f32> =
            (0..8192).map(|n| (0.25 * (2.0 * std::f64::consts::PI * f * n as f64 / rate).sin()) as f32).collect();
        let mut sp = Spectrum::new(4096);
        let bins = sp.analyse(&s, 0.0).to_vec();
        let k = (f / rate * 4096.0).round() as usize;
        let (kmax, &vmax) = bins.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap();
        assert_eq!(kmax, k);
        // Hann scalloping at an exact bin is 0: −12 dBFS within 0.1 dB.
        assert!((vmax - 20.0 * 0.25f64.log10()).abs() < 0.1, "{vmax}");
    }
}
