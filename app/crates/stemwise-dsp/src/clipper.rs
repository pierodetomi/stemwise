//! Peak clipper with 4x oversampling (MAS-11).
//!
//! **Algorithm.** Per sample the signal goes through [`Oversampler4`] (two half-band stages, linear phase, see that
//! module) and the shaper is applied to each of the four points at the 4x rate, so the harmonics created by the
//! corner fold back far less than at the base rate. With threshold `T` and knee start `a = T·(1 − 0.5·softness)`,
//! the shaper is odd-symmetric:
//! - `|x| ≤ a`: `|x|` (unity, no level compensation);
//! - above: `T − (T − a)·exp(−(|x| − a)/(T − a))`, C1-continuous at `a` (slope 1) with asymptote `T`;
//! - `softness = 0`: hard clip `min(|x|, T)`.
//!
//! The output therefore never exceeds `T` at the 4x rate. After decimation the half-band filter's Gibbs overshoot
//! can add a fraction of a dB (the stopband edge sits near Nyquist, so a flat-top at 4x is not exactly
//! representable at the base rate); the tests allow 0.5 dB.
//!
//! **Smoothing.** `threshold` and `softness` are smoothed per sample (5 ms one-pole), independently per channel,
//! like [`crate::saturation::Saturation`]; output is independent of block size. [`Clipper::settle`] jumps to the
//! targets (offline render start).
//!
//! **Latency.** [`Clipper::LATENCY`] = [`Oversampler4::LATENCY`] = 64 samples, integer, compensated by the engine.
//!
//! **Meter.** [`Clipper::take_gr_db`] is the deepest gain `|y|/|x|` applied to any 4x sample since the last call,
//! over both channels (≤ 0 dB; 0 when nothing went above the knee).
//!
//! **Properties verified (tests).** Exact latency, transparency below the knee (±0.01 dB), bounded peak, meter
//! value on a known peak, lower 3rd harmonic with softer shaping, aliasing of 15 kHz at 48 kHz below −50 dB,
//! block-size invariance (≤ 1e-6), independent channels, no NaN/inf at threshold 0.01 on full-scale noise.
//! `process` does not allocate, lock or perform I/O.

use crate::oversample::Oversampler4;
use crate::util::{flush, one_pole};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipperParams {
    /// Linear amplitude the output must not exceed at the 4x rate, > 0.
    pub threshold: f64,
    /// 0 = hard … 1 = softest.
    pub softness: f64,
}

impl Default for ClipperParams {
    fn default() -> Self {
        ClipperParams { threshold: 1.0, softness: 0.4 }
    }
}

const SMOOTH_MS: f64 = 5.0;
const MIN_THRESHOLD: f64 = 1e-6;

#[derive(Debug, Clone)]
struct Chan {
    os: Oversampler4,
    thr: f64,
    soft: f64,
    /// Deepest linear gain applied to a 4x sample since the last `take_gr_db` (1 = none).
    min_gain: f32,
}

impl Chan {
    fn new(p: &ClipperParams) -> Self {
        Chan { os: Oversampler4::new(), thr: p.threshold, soft: p.softness, min_gain: 1.0 }
    }
}

#[derive(Debug, Clone)]
pub struct Clipper {
    ch: [Chan; 2],
    target: ClipperParams,
    coef: f64,
}

/// The shaper on the four 4x points; records the deepest gain.
#[inline]
fn shape4(v: &mut [f32; 4], t: f32, soft: f32, min_gain: &mut f32) {
    let a = t * (1.0 - 0.5 * soft);
    let w = t - a;
    for x in v.iter_mut() {
        let m = x.abs();
        if m <= a {
            continue;
        }
        let y = if w <= 1e-9 * t { t } else { t - w * (-(m - a) / w).exp() };
        *min_gain = min_gain.min(y / m);
        *x = y.copysign(*x);
    }
}

impl Clipper {
    pub const LATENCY: usize = Oversampler4::LATENCY;

    pub fn new(rate: f64) -> Self {
        let p = ClipperParams::default();
        Clipper { ch: [Chan::new(&p), Chan::new(&p)], target: p, coef: one_pole(SMOOTH_MS, rate) }
    }

    pub fn set_params(&mut self, p: &ClipperParams) {
        self.target = ClipperParams { threshold: p.threshold.max(MIN_THRESHOLD), softness: p.softness.clamp(0.0, 1.0) };
    }

    /// Jumps the smoothing to its targets (start of an offline render).
    pub fn settle(&mut self) {
        for c in &mut self.ch {
            c.thr = self.target.threshold;
            c.soft = self.target.softness;
        }
    }

    pub fn reset(&mut self) {
        for c in &mut self.ch {
            c.os.reset();
            c.min_gain = 1.0;
        }
        self.settle();
    }

    /// Deepest gain applied to a 4x sample since the last call, in dB (≤ 0), over both channels.
    pub fn take_gr_db(&mut self) -> f64 {
        let g = self.ch[0].min_gain.min(self.ch[1].min_gain);
        self.ch[0].min_gain = 1.0;
        self.ch[1].min_gain = 1.0;
        20.0 * f64::from(g).log10().min(0.0)
    }

    /// In place, channel `ch` (0 or 1). Output is delayed by [`Self::LATENCY`] samples.
    pub fn process(&mut self, ch: usize, buf: &mut [f32]) {
        let c = &mut self.ch[ch];
        let (tt, ts, k) = (self.target.threshold, self.target.softness, self.coef);
        for s in buf.iter_mut() {
            c.thr += k * (tt - c.thr);
            c.soft += k * (ts - c.soft);
            if (tt - c.thr).abs() < 1e-9 * tt {
                c.thr = tt;
            }
            if (ts - c.soft).abs() < 1e-9 {
                c.soft = ts;
            }
            let (t, soft) = (c.thr as f32, c.soft as f32);
            let mut mg = c.min_gain;
            let y = c.os.process4(*s, |v| shape4(v, t, soft, &mut mg));
            c.min_gain = mg;
            *s = flush(f64::from(y)) as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::db_to_gain;

    const FS: f64 = 48000.0;
    const TAU: f64 = 2.0 * std::f64::consts::PI;

    fn sine(freq: f64, db: f64, n: usize) -> Vec<f32> {
        let a = db_to_gain(db);
        (0..n).map(|t| (a * (TAU * freq * t as f64 / FS).sin()) as f32).collect()
    }

    fn make(threshold: f64, softness: f64) -> Clipper {
        let mut c = Clipper::new(FS);
        c.set_params(&ClipperParams { threshold, softness });
        c.settle();
        c
    }

    fn peak(x: &[f32]) -> f64 {
        x.iter().fold(0.0f64, |a, v| a.max(f64::from(v.abs())))
    }

    fn rms(x: &[f32]) -> f64 {
        (x.iter().map(|v| f64::from(*v).powi(2)).sum::<f64>() / x.len() as f64).sqrt()
    }

    fn bin(x: &[f32], freq: f64) -> f64 {
        let (mut re, mut im) = (0.0, 0.0);
        for (t, v) in x.iter().enumerate() {
            let ph = TAU * freq * t as f64 / FS;
            re += f64::from(*v) * ph.cos();
            im += f64::from(*v) * ph.sin();
        }
        2.0 * (re * re + im * im).sqrt() / x.len() as f64
    }

    /// req: MAS-11
    #[test]
    fn latency_exact() {
        let mut c = make(1.0, 0.4);
        let mut x = vec![0.0f32; 300];
        x[0] = 0.01;
        c.process(0, &mut x);
        let p = x.iter().enumerate().max_by(|a, b| a.1.abs().total_cmp(&b.1.abs())).unwrap().0;
        assert_eq!(p, Clipper::LATENCY);
        assert_eq!(Clipper::LATENCY, Oversampler4::LATENCY);
    }

    /// req: MAS-11
    #[test]
    fn transparent_below_the_knee() {
        // Threshold 1, softness 0.4: knee at 0.8. A -3 dBFS sine (0.708) peaks below it even with the
        // oversampler's tiny ripple; the 0.01 dB tolerance is the half-band passband ripple budget.
        let mut c = make(1.0, 0.4);
        let inp = sine(1000.0, -3.0, 48000);
        let mut y = inp.clone();
        c.process(0, &mut y);
        let d = 20.0 * (rms(&y[2000..]) / rms(&inp[2000..])).log10();
        assert!(d.abs() < 0.01, "{d} dB");
        assert_eq!(c.take_gr_db(), 0.0);
    }

    /// req: MAS-11
    #[test]
    fn peak_is_bounded() {
        // The shaper bounds the 4x signal at T exactly; the half-band decimator may overshoot a flat-topped
        // wave by a Gibbs ripple of a fraction of a dB, so the base-rate sample peak is allowed T + 0.5 dB.
        for soft in [0.0, 0.4, 1.0] {
            let mut c = make(0.5, soft);
            let mut y = sine(997.0, -6.0 + 12.0, 48000);
            c.process(0, &mut y);
            let p = 20.0 * (peak(&y[2000..]) / 0.5).log10();
            assert!(p <= 0.5, "softness {soft}: {p} dB over");
        }
    }

    /// req: MAS-11
    #[test]
    fn gain_reduction_meter() {
        // Hard clip of a 2.0 sine at T = 1: the 4x grid sees a peak within 0.5% of 2.0, reduction 6.02 dB.
        let mut c = make(1.0, 0.0);
        let mut y = sine(1000.0, 20.0 * 2.0f64.log10(), 9600);
        c.process(0, &mut y);
        let gr = c.take_gr_db();
        assert!((gr + 6.02).abs() < 0.3, "{gr}");
        assert_eq!(c.take_gr_db(), 0.0, "meter is taken");
        // Soft: at x = 2, T = 1, a = 0.8: y = 1 - 0.2 exp(-6) = 0.99950: -6.02 dB as well.
        let mut c = make(1.0, 0.4);
        let mut y = sine(1000.0, 20.0 * 2.0f64.log10(), 9600);
        c.process(0, &mut y);
        assert!((c.take_gr_db() + 6.02).abs() < 0.3);
    }

    /// req: MAS-11
    #[test]
    fn softer_knee_has_less_third_harmonic() {
        let h3 = |soft: f64| {
            let mut c = make(0.5, soft);
            let mut y = sine(1000.0, -6.0 + 6.0, 48000 + 2000);
            c.process(0, &mut y);
            let y = &y[2000..];
            bin(y, 3000.0) / bin(y, 1000.0)
        };
        let (hard, soft) = (h3(0.0), h3(1.0));
        assert!(soft < hard, "soft {soft} hard {hard}");
    }

    /// req: MAS-11
    #[test]
    fn aliasing_low() {
        let mut c = make(0.5, 0.4);
        let mut y = sine(15000.0, -6.0, 48000 + 2000);
        c.process(0, &mut y);
        let y = &y[2000..];
        // 3rd harmonic 45 kHz folds to 3 kHz at 48 kHz.
        let rel = 20.0 * (bin(y, 3000.0) / bin(y, 15000.0)).log10();
        assert!(rel < -50.0, "alias {rel} dB");
    }

    /// req: MAS-11
    #[test]
    fn block_size_invariant() {
        let inp = sine(440.0, 0.0, 16384);
        let run = |block: usize| {
            let mut c = Clipper::new(FS);
            let mut out = inp.clone();
            // Parameters change every 4096 samples, at the same positions for every block size.
            for (i, seg) in out.chunks_mut(4096).enumerate() {
                let k = i as f64;
                c.set_params(&ClipperParams { threshold: 0.9 - 0.15 * k, softness: 0.1 * k });
                for chunk in seg.chunks_mut(block) {
                    c.process(0, chunk);
                }
            }
            out
        };
        let (a, b, d) = (run(1), run(64), run(1000));
        let worst = |p: &[f32], q: &[f32]| p.iter().zip(q).map(|(x, y)| (x - y).abs()).fold(0.0f32, f32::max);
        assert!(worst(&a, &b) <= 1e-6 && worst(&a, &d) <= 1e-6, "{} {}", worst(&a, &b), worst(&a, &d));
    }

    /// req: MAS-11
    #[test]
    fn channels_independent() {
        let mut c = make(0.5, 0.4);
        let mut l = sine(1000.0, 0.0, 4000);
        let mut r = vec![0.0f32; 4000];
        c.process(0, &mut l);
        c.process(1, &mut r);
        assert!(r.iter().all(|v| *v == 0.0));
        let mut c2 = make(0.5, 0.4);
        let mut l2 = sine(1000.0, 0.0, 4000);
        c2.process(0, &mut l2);
        assert_eq!(l, l2);
    }

    /// req: MAS-11
    #[test]
    fn no_nan_on_noise() {
        let mut c = make(0.01, 0.4);
        let mut x: Vec<f32> = crate::testsignal::xorshift32(7, 20000).into_iter().map(|v| v as f32).collect();
        c.process(0, &mut x);
        assert!(x.iter().all(|v| v.is_finite() && v.abs() < 0.02));
        let gr = c.take_gr_db();
        assert!(gr.is_finite() && gr < 0.0);
    }
}
