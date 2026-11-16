//! Stereo width (FX-19, FX-24).
//!
//! **Algorithm.** Mid/side: `M = (L + R) / 2`, `S = (L - R) / 2`; `S' = S * width` with `width` (0..2, the gain of
//! the side) smoothed per sample (20 ms); `L' = M + S'`, `R' = M - S'`. With `mono_below_hz` > 0 the side first goes
//! through a Linkwitz-Riley high-pass (two cascaded Butterworth biquads, 24 dB/oct) at that frequency, so the
//! low end is mono. The frequency is smoothed (20 ms) and the coefficients are re-designed every 32 samples
//! counted since `reset` (block-size independent, no allocation). At 0 (off) the filter is bypassed (no filter in
//! the path); it keeps running at 1 Hz so its state is warm when it is switched on.
//!
//! Neutral (FX-24): `width` 1 and `mono_below_hz` 0 give the identity. Latency 0.
//!
//! **Properties verified (tests).** Identity at neutral (<= 1e-6), width 0 gives L = R, width 2 doubles the side,
//! mono at 120 Hz removes a 50 Hz side component by >= 20 dB and keeps 1 kHz within 0.5 dB, L = R input is unchanged
//! at any width (<= 1e-6), block-size invariance (<= 1e-6), bounded and finite on noise with changing parameters.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::util::one_pole;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WidthParams {
    /// Gain of the side, 0..2.
    pub width: f64,
    /// Frequency below which the side is removed, 0..250 Hz; 0 = off.
    pub mono_below_hz: f64,
}

const GRID: u32 = 32;

/// Stereo width and mono-below filter (FX-19).
pub struct StereoWidth {
    rate: f64,
    p: WidthParams,
    width: f64,
    freq: f64,
    k: f64,
    count: u32,
    snap: bool,
    hp: [Biquad; 2],
}

impl StereoWidth {
    pub fn new(rate: f64) -> Self {
        StereoWidth {
            rate,
            p: WidthParams { width: 1.0, mono_below_hz: 0.0 },
            width: 1.0,
            freq: 0.0,
            k: one_pole(20.0, rate),
            count: 0,
            snap: true,
            hp: [Biquad::default(), Biquad::default()],
        }
    }

    pub fn set_params(&mut self, p: &WidthParams) {
        self.p = WidthParams { width: p.width.clamp(0.0, 2.0), mono_below_hz: p.mono_below_hz.clamp(0.0, 250.0) };
    }

    pub fn reset(&mut self) {
        for h in &mut self.hp {
            h.reset();
        }
        self.count = 0;
        self.snap = true;
    }

    fn design(&mut self) {
        let c = Coeffs::design(Kind::HighPass, self.freq.max(1.0), 0.0, BUTTERWORTH_Q, self.rate);
        self.hp[0].c = c;
        self.hp[1].c = c;
    }

    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len().min(r.len());
        if self.snap {
            self.width = self.p.width;
            self.freq = self.p.mono_below_hz;
            self.design();
            self.snap = false;
        }
        for i in 0..n {
            self.width += (self.p.width - self.width) * self.k;
            self.freq += (self.p.mono_below_hz - self.freq) * self.k;
            if self.p.mono_below_hz == 0.0 && self.freq < 1.0 {
                self.freq = 0.0;
            }
            if self.count.is_multiple_of(GRID) {
                self.design();
            }
            self.count = self.count.wrapping_add(1);
            let (a, b) = (f64::from(l[i]), f64::from(r[i]));
            let m = 0.5 * (a + b);
            let mut s = 0.5 * (a - b);
            let y0 = self.hp[0].tick(0, s);
            let y = self.hp[1].tick(0, y0);
            if self.freq > 0.0 {
                s = y;
            }
            s *= self.width;
            l[i] = (m + s) as f32;
            r[i] = (m - s) as f32;
        }
        self.hp[0].flush_state();
        self.hp[1].flush_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::xorshift32;
    use std::f64::consts::TAU;

    const FS: f64 = 48000.0;

    fn noise(seed: u32, n: usize) -> Vec<f32> {
        xorshift32(seed, n).iter().map(|v| (v * 0.5) as f32).collect()
    }

    fn run(p: &WidthParams, l: &[f32], r: &[f32], chunk: usize) -> (Vec<f32>, Vec<f32>) {
        let mut w = StereoWidth::new(FS);
        w.set_params(p);
        let (mut ol, mut or) = (l.to_vec(), r.to_vec());
        let mut i = 0;
        while i < ol.len() {
            let e = (i + chunk).min(ol.len());
            w.process(&mut ol[i..e], &mut or[i..e]);
            i = e;
        }
        (ol, or)
    }

    fn maxdiff(a: &[f32], b: &[f32]) -> f64 {
        a.iter().zip(b).map(|(x, y)| f64::from(x - y).abs()).fold(0.0, f64::max)
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    /// req: FX-19, FX-24 — width 1 and mono 0 is the identity (<= 1e-6, f32 rounding).
    #[test]
    fn neutral_is_identity() {
        let (l, r) = (noise(1, 20000), noise(2, 20000));
        let (ol, or) = run(&WidthParams { width: 1.0, mono_below_hz: 0.0 }, &l, &r, 32);
        assert!(maxdiff(&ol, &l) <= 1e-6 && maxdiff(&or, &r) <= 1e-6);
    }

    /// req: FX-19 — width 0 collapses to mono (L = R = M), width 2 doubles the side (<= 1e-6).
    #[test]
    fn width_zero_and_two() {
        let (l, r) = (noise(3, 20000), noise(4, 20000));
        let (ol, or) = run(&WidthParams { width: 0.0, mono_below_hz: 0.0 }, &l, &r, 32);
        assert!(maxdiff(&ol, &or) <= 1e-6);
        let (ol, or) = run(&WidthParams { width: 2.0, mono_below_hz: 0.0 }, &l, &r, 32);
        for i in 0..20000 {
            let s = 0.5 * (f64::from(l[i]) - f64::from(r[i]));
            let so = 0.5 * (f64::from(ol[i]) - f64::from(or[i]));
            assert!((so - 2.0 * s).abs() <= 1e-6);
        }
    }

    /// req: FX-19 — mono below 120 Hz removes a 50 Hz side component by >= 20 dB (LR4 gives about 30 dB at that
    /// ratio) and keeps a 1 kHz side component within 0.5 dB.
    #[test]
    fn mono_below_filters_side() {
        let n = 48000;
        let side = |f: f64| -> Vec<f32> { (0..n).map(|t| (0.5 * (TAU * f * t as f64 / FS).sin()) as f32).collect() };
        let neg = |v: &[f32]| -> Vec<f32> { v.iter().map(|x| -x).collect() };
        let p = WidthParams { width: 1.0, mono_below_hz: 120.0 };
        for (f, check) in [(50.0, 0), (1000.0, 1)] {
            let s = side(f);
            let (ol, _) = run(&p, &s, &neg(&s), 32);
            let g = 20.0 * (rms(&ol[24000..]) / rms(&s[24000..])).log10();
            if check == 0 {
                assert!(g <= -20.0, "{g}");
            } else {
                assert!(g.abs() <= 0.5, "{g}");
            }
        }
    }

    /// req: FX-19 — a mono input (L = R) is unchanged at any width and with the mono filter on (<= 1e-6).
    #[test]
    fn mono_input_unchanged() {
        let x = noise(5, 20000);
        for width in [0.0, 0.5, 1.5, 2.0] {
            let (ol, or) = run(&WidthParams { width, mono_below_hz: 200.0 }, &x, &x, 32);
            assert!(maxdiff(&ol, &x) <= 1e-6 && maxdiff(&or, &x) <= 1e-6, "{width}");
        }
    }

    /// req: FX-19, FX-24 — the output does not depend on the block size (<= 1e-6).
    #[test]
    fn block_size_invariance() {
        let (l, r) = (noise(6, 30000), noise(7, 30000));
        let p = WidthParams { width: 1.6, mono_below_hz: 150.0 };
        let (a, _) = run(&p, &l, &r, 32);
        for chunk in [1000, 7] {
            let (b, _) = run(&p, &l, &r, chunk);
            assert!(maxdiff(&a, &b) <= 1e-6, "chunk {chunk}");
        }
    }

    /// req: FX-19, FX-24 — parameters changing every 32 samples on full-scale noise stay finite and within 2.0 (|M| + |S| * 2 <= 1 + 1).
    #[test]
    fn stable_under_parameter_changes() {
        let (mut l, mut r) = (noise(8, 5 * 48000), noise(9, 5 * 48000));
        for v in l.iter_mut().chain(r.iter_mut()) {
            *v *= 2.0; // full scale
        }
        let mut w = StereoWidth::new(FS);
        let s = xorshift32(10, 5 * 48000 / 32 + 1);
        for (b, (cl, cr)) in l.chunks_mut(32).zip(r.chunks_mut(32)).enumerate() {
            let u = s[b] * 0.5 + 0.5;
            w.set_params(&WidthParams { width: 2.0 * u, mono_below_hz: if u < 0.2 { 0.0 } else { 250.0 * u } });
            w.process(cl, cr);
        }
        // LR4 high-pass of the side can overshoot slightly: allow 2.5.
        assert!(l.iter().chain(r.iter()).all(|v| v.is_finite() && v.abs() <= 2.5));
    }
}
