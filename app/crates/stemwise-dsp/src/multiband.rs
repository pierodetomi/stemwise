//! Multiband compressor with 2 to 6 bands (MAS-12).
//!
//! **Crossover.** 4th-order Linkwitz-Riley: each slope is two cascaded 2nd-order Butterworth biquads (Q = 1/√2).
//! The bands form a cascade ("tree") that splits the lowest band off first. With crossovers `f0 < f1 < … < f(N−2)`:
//!
//! ```text
//! rest₀ = x
//! band j   = LP(fj)(rest_j)        (j = 0 … N−2)   then through AP(f(j+1)) … AP(f(N−2))
//! rest(j+1) = HP(fj)(rest_j)
//! band N−1 = rest(N−1)
//! ```
//!
//! **Compensation.** LP4 + HP4 of a Linkwitz-Riley pair at the same frequency is the 2nd-order all-pass
//! `(s² − √2 s + 1)/(s² + √2 s + 1)` (Q = 1/√2). A band split off at `fj` has not passed through the splits at
//! `f(j+1)…`, which the bands above it did, so it goes through that all-pass at each later crossover. By induction the
//! sum of all bands is `AP(f0)·AP(f1)·…·AP(f(N−2))·x`, an all-pass of the input: flat magnitude for every N.
//! (For N = 3: `B0 = LP0·AP1`, `B1 = LP1·HP0`, `B2 = HP1·HP0`; `B1 + B2 = AP1·HP0` and `B0 + B1 + B2 = AP1·(LP0 + HP0) =
//! AP1·AP0`.)
//!
//! **Compression.** Each band goes through one stereo-linked [`Compressor`]; a band that is not `on` passes
//! uncompressed. If any band has `solo`, only the solo bands are summed. Zero latency (IIR).
//!
//! **Real time.** `new` allocates everything for 6 bands (fixed arrays and the compressors' own state); `set_params`
//! and `process` never allocate: bands are computed in chunks of [`CHUNK`] samples into preallocated buffers.
//! Crossover frequencies are clamped to 20 Hz … 0.45·rate and kept ascending with at least 1/3 octave between
//! neighbours. Changing the band count resets the filters; changing frequencies only swaps coefficients.
//!
//! **Properties verified (tests).** Flat sum (±0.1 dB, 30 Hz–18 kHz) for 2…6 bands, band isolation, solo,
//! crossover changes without NaN, block-size invariance (≤ 1e-6), stereo linking, no allocation (see
//! `tests/no_alloc.rs`).

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::compressor::{Compressor, CompressorParams};

pub const MAX_BANDS: usize = 6;
const XO: usize = MAX_BANDS - 1;
/// Samples processed per internal pass (size of the preallocated band buffers).
pub const CHUNK: usize = 256;
const MIN_XO_HZ: f64 = 20.0;
const MAX_XO_RATE: f64 = 0.45;
/// Minimum distance between neighbouring crossovers: 1/3 octave.
const MIN_RATIO: f64 = 1.259_921_049_894_873_2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BandParams {
    pub on: bool,
    pub solo: bool,
    pub comp: CompressorParams,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultibandParams {
    /// 2..=6.
    pub bands: usize,
    /// The first `bands − 1` are used, ascending.
    pub xo_hz: [f64; MAX_BANDS - 1],
    pub band: [BandParams; MAX_BANDS],
}

impl Default for MultibandParams {
    fn default() -> Self {
        let comp = CompressorParams { ratio: 1.0, ..Default::default() };
        MultibandParams {
            bands: 3,
            xo_hz: [120.0, 4000.0, 8000.0, 12000.0, 16000.0],
            band: [BandParams { on: true, solo: false, comp }; MAX_BANDS],
        }
    }
}

type Lr4 = [Biquad; 2];

pub struct Multiband {
    rate: f64,
    n: usize,
    xo: [f64; XO],
    lp: [Lr4; XO],
    hp: [Lr4; XO],
    /// `ap[band][crossover]`, used for `crossover > band`.
    ap: [[Biquad; XO]; XO],
    comp: [Compressor; MAX_BANDS],
    on: [bool; MAX_BANDS],
    solo: [bool; MAX_BANDS],
    bl: [[f32; CHUNK]; MAX_BANDS],
    br: [[f32; CHUNK]; MAX_BANDS],
}

fn allpass(freq: f64, rate: f64) -> Coeffs {
    let w0 = 2.0 * std::f64::consts::PI * freq / rate;
    let (sin, cos) = w0.sin_cos();
    let alpha = sin / (2.0 * BUTTERWORTH_Q);
    let a0 = 1.0 + alpha;
    Coeffs {
        b0: (1.0 - alpha) / a0,
        b1: -2.0 * cos / a0,
        b2: (1.0 + alpha) / a0,
        a1: -2.0 * cos / a0,
        a2: (1.0 - alpha) / a0,
    }
}

impl Multiband {
    pub fn new(rate: f64) -> Self {
        let mut m = Multiband {
            rate,
            n: 0,
            xo: [0.0; XO],
            lp: Default::default(),
            hp: Default::default(),
            ap: Default::default(),
            comp: std::array::from_fn(|_| Compressor::new(rate)),
            on: [true; MAX_BANDS],
            solo: [false; MAX_BANDS],
            bl: [[0.0; CHUNK]; MAX_BANDS],
            br: [[0.0; CHUNK]; MAX_BANDS],
        };
        m.set_params(&MultibandParams::default());
        m
    }

    /// Frequencies clamped to the valid range and made ascending with a 1/3-octave gap.
    fn conform(&self, n: usize, xo: &[f64; XO]) -> [f64; XO] {
        let hi = self.rate * MAX_XO_RATE;
        let mut out = [0.0; XO];
        let mut prev = 0.0;
        for j in 0..n.saturating_sub(1) {
            let lo = MIN_XO_HZ * MIN_RATIO.powi(j as i32);
            let top = hi / MIN_RATIO.powi((n - 2 - j) as i32);
            let f = if xo[j].is_finite() { xo[j] } else { lo };
            let f = f.max(prev * MIN_RATIO).max(lo).min(top);
            out[j] = f;
            prev = f;
        }
        out
    }

    pub fn set_params(&mut self, p: &MultibandParams) {
        let n = p.bands.clamp(2, MAX_BANDS);
        let xo = self.conform(n, &p.xo_hz);
        if n != self.n {
            self.n = n;
            self.reset_filters();
            self.xo = [0.0; XO];
        }
        for j in 0..n - 1 {
            if xo[j] != self.xo[j] {
                let lp = Coeffs::design(Kind::LowPass, xo[j], 0.0, BUTTERWORTH_Q, self.rate);
                let hp = Coeffs::design(Kind::HighPass, xo[j], 0.0, BUTTERWORTH_Q, self.rate);
                let ap = allpass(xo[j], self.rate);
                for s in 0..2 {
                    self.lp[j][s].c = lp;
                    self.hp[j][s].c = hp;
                }
                for b in 0..j {
                    self.ap[b][j].c = ap;
                }
                self.xo[j] = xo[j];
            }
        }
        for b in 0..n {
            self.on[b] = p.band[b].on;
            self.solo[b] = p.band[b].solo;
            self.comp[b].set_params(&p.band[b].comp);
        }
    }

    fn reset_filters(&mut self) {
        for j in 0..XO {
            for s in 0..2 {
                self.lp[j][s].reset();
                self.hp[j][s].reset();
            }
            for k in 0..XO {
                self.ap[j][k].reset();
            }
        }
    }

    pub fn reset(&mut self) {
        self.reset_filters();
        for c in &mut self.comp {
            c.reset();
        }
    }

    /// Deepest reduction per band (dB, ≤ 0) since the last call; bands beyond the current count and bands that
    /// are off read 0.
    pub fn take_gr_db(&mut self) -> [f64; MAX_BANDS] {
        let mut out = [0.0; MAX_BANDS];
        for b in 0..MAX_BANDS {
            let g = self.comp[b].take_gr_db();
            out[b] = if b < self.n && self.on[b] { g } else { 0.0 };
        }
        out
    }

    /// In place, any block size, zero latency.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let len = l.len().min(r.len());
        let n = self.n;
        let any_solo = self.solo[..n].iter().any(|s| *s);
        let mut pos = 0;
        while pos < len {
            let m = CHUNK.min(len - pos);
            for t in 0..m {
                for (ch, src) in [&l[pos + t], &r[pos + t]].into_iter().enumerate() {
                    let mut rest = f64::from(*src);
                    for j in 0..n - 1 {
                        let lo0 = self.lp[j][0].tick(ch, rest);
                        let mut lo = self.lp[j][1].tick(ch, lo0);
                        let hi0 = self.hp[j][0].tick(ch, rest);
                        rest = self.hp[j][1].tick(ch, hi0);
                        for k in j + 1..n - 1 {
                            lo = self.ap[j][k].tick(ch, lo);
                        }
                        if ch == 0 {
                            self.bl[j][t] = lo as f32;
                        } else {
                            self.br[j][t] = lo as f32;
                        }
                    }
                    if ch == 0 {
                        self.bl[n - 1][t] = rest as f32;
                    } else {
                        self.br[n - 1][t] = rest as f32;
                    }
                }
            }
            for b in 0..n {
                let (bl, br) = (&mut self.bl[b][..m], &mut self.br[b][..m]);
                if self.on[b] {
                    self.comp[b].process(&mut [bl, br], None);
                }
            }
            for t in 0..m {
                let (mut sl, mut sr) = (0.0f64, 0.0f64);
                for b in 0..n {
                    if !any_solo || self.solo[b] {
                        sl += f64::from(self.bl[b][t]);
                        sr += f64::from(self.br[b][t]);
                    }
                }
                l[pos + t] = sl as f32;
                r[pos + t] = sr as f32;
            }
            for j in 0..XO {
                for s in 0..2 {
                    self.lp[j][s].flush_state();
                    self.hp[j][s].flush_state();
                }
                for k in 0..XO {
                    self.ap[j][k].flush_state();
                }
            }
            pos += m;
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

    fn add(a: &[f32], b: &[f32]) -> Vec<f32> {
        a.iter().zip(b).map(|(x, y)| x + y).collect()
    }

    /// Amplitude of `freq` in `x` by projection; exact for whole cycles in the window.
    fn bin(x: &[f32], freq: f64, offset: usize) -> f64 {
        let (mut re, mut im) = (0.0, 0.0);
        for (t, v) in x.iter().enumerate() {
            let ph = TAU * freq * (t + offset) as f64 / FS;
            re += f64::from(*v) * ph.cos();
            im += f64::from(*v) * ph.sin();
        }
        2.0 * (re * re + im * im).sqrt() / x.len() as f64
    }

    fn params(n: usize, xo: &[f64]) -> MultibandParams {
        let mut p = MultibandParams { bands: n, ..Default::default() };
        p.xo_hz[..xo.len()].copy_from_slice(xo);
        p
    }

    fn run(p: &MultibandParams, l: &[f32], r: &[f32], block: usize) -> (Vec<f32>, Vec<f32>, [f64; MAX_BANDS]) {
        let mut m = Multiband::new(FS);
        m.set_params(p);
        let (mut ol, mut or) = (l.to_vec(), r.to_vec());
        for (a, b) in ol.chunks_mut(block).zip(or.chunks_mut(block)) {
            m.process(a, b);
        }
        (ol, or, m.take_gr_db())
    }

    /// req: MAS-12
    #[test]
    fn sum_of_bands_is_flat() {
        // Frequencies are even integers so a 0.5 s window holds whole cycles. The all-pass sum is exactly flat in
        // theory; the 0.1 dB tolerance covers the transient tail of the 30 Hz and the f32 band buffers.
        let xos: [&[f64]; 5] = [
            &[1000.0],
            &[120.0, 4000.0],
            &[100.0, 800.0, 5000.0],
            &[80.0, 400.0, 2000.0, 8000.0],
            &[60.0, 250.0, 1000.0, 3500.0, 9000.0],
        ];
        for (i, xo) in xos.iter().enumerate() {
            let n = i + 2;
            let p = params(n, xo);
            for k in 0..30 {
                let f = (30.0 * (18000.0f64 / 30.0).powf(k as f64 / 29.0) / 2.0).round() * 2.0;
                let x = sine(f, -6.0, 72000);
                let (y, _, _) = run(&p, &x, &x, 512);
                let d = 20.0 * (bin(&y[48000..], f, 48000) / bin(&x[48000..], f, 48000)).log10();
                assert!(d.abs() < 0.1, "{n} bands, {f} Hz: {d} dB");
            }
        }
    }

    /// req: MAS-12
    #[test]
    fn low_band_compression_leaves_the_others_alone() {
        let mut p = params(3, &[120.0, 4000.0]);
        p.band[0].comp = CompressorParams {
            threshold_db: -30.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 500.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let low = sine(50.0, -6.0, 96000);
        let high = sine(5000.0, -20.0, 96000);
        let x = add(&low, &high);
        let (y, _, gr) = run(&p, &x, &x, 480);
        assert!(gr[0] < -5.0, "{gr:?}");
        assert!(gr[1..].iter().all(|g| *g == 0.0), "{gr:?}");
        let d = 20.0 * (bin(&y[48000..], 5000.0, 48000) / bin(&x[48000..], 5000.0, 48000)).log10();
        assert!(d.abs() < 0.2, "{d} dB");
        let dl = 20.0 * (bin(&y[48000..], 50.0, 48000) / bin(&x[48000..], 50.0, 48000)).log10();
        assert!(dl < -3.0, "{dl} dB");
    }

    /// req: MAS-12
    #[test]
    fn solo_returns_only_that_band() {
        // The soloed band's own passband loses < 0.1 dB at 30 Hz (LR4 at 1/4 of the crossover).
        // LR4 is 24.6 dB down one octave past its crossover (1/(1 + 2^4)): more than 24 dB.
        let level = |p: &MultibandParams, f: f64| {
            let x = sine(f, -6.0, 48000);
            let (y, _, _) = run(p, &x, &x, 256);
            20.0 * (bin(&y[24000..], f, 24000) / bin(&x[24000..], f, 24000)).log10()
        };
        let mut p = params(3, &[120.0, 4000.0]);
        p.band[0].solo = true;
        assert!(level(&p, 240.0) < -24.0);
        assert!(level(&p, 30.0).abs() < 0.2);
        let mut p = params(3, &[120.0, 4000.0]);
        p.band[2].solo = true;
        assert!(level(&p, 2000.0) < -24.0);
        assert!(level(&p, 10000.0).abs() < 0.2);
        // Two solos sum; a band that is not on is passed uncompressed but still summed.
        let mut p = params(3, &[120.0, 4000.0]);
        p.band[0].solo = true;
        p.band[2].solo = true;
        assert!(level(&p, 30.0).abs() < 0.2 && level(&p, 10000.0).abs() < 0.2 && level(&p, 1000.0) < -12.0);
    }

    /// req: MAS-12
    #[test]
    fn band_off_passes_uncompressed() {
        let mut p = params(2, &[1000.0]);
        p.band[0].comp = CompressorParams { threshold_db: -40.0, ratio: 10.0, ..Default::default() };
        p.band[0].on = false;
        let x = sine(100.0, -6.0, 48000);
        let (y, _, gr) = run(&p, &x, &x, 256);
        assert_eq!(gr[0], 0.0);
        let d = 20.0 * (bin(&y[24000..], 100.0, 24000) / bin(&x[24000..], 100.0, 24000)).log10();
        assert!(d.abs() < 0.1, "{d}");
    }

    /// req: MAS-12
    #[test]
    fn crossover_changes_never_produce_nan() {
        let noise: Vec<f32> = crate::testsignal::xorshift32(5, 96000).iter().map(|v| (*v * 0.9) as f32).collect();
        let freqs = crate::testsignal::xorshift32(6, 400);
        let mut m = Multiband::new(FS);
        let (mut l, mut r) = (noise.clone(), noise);
        for (i, (a, b)) in l.chunks_mut(240).zip(r.chunks_mut(240)).enumerate() {
            let mut p = MultibandParams { bands: 2 + i % 5, ..Default::default() };
            for j in 0..5 {
                // Wild values, including out of range and out of order.
                p.xo_hz[j] = 10.0 + (freqs[(i * 5 + j) % 400] + 1.0) * 15000.0;
            }
            if i % 7 == 0 {
                p.xo_hz[0] = 1.0e9;
            }
            p.band[1].comp.ratio = 4.0;
            p.band[1].comp.threshold_db = -30.0;
            m.set_params(&p);
            m.process(a, b);
        }
        assert!(l.iter().chain(&r).all(|v| v.is_finite() && v.abs() < 8.0));
    }

    /// req: MAS-12
    #[test]
    fn crossovers_are_clamped_and_ordered() {
        let m = Multiband::new(FS);
        let f = m.conform(4, &[5.0, 5.0, 1.0e6, 0.0, 0.0]);
        assert!(f[0] >= 20.0);
        assert!(f[1] >= f[0] * 1.259 && f[2] >= f[1] * 1.259);
        assert!(f[2] <= 0.45 * FS + 1e-9);
        let g = m.conform(6, &[1.0e6; 5]);
        assert!(g.windows(2).all(|w| w[1] >= w[0] * 1.259) && g[4] <= 0.45 * FS + 1e-9);
    }

    /// req: MAS-12
    #[test]
    fn block_size_invariant() {
        let mut p = params(4, &[100.0, 700.0, 3000.0]);
        for b in 0..4 {
            p.band[b].comp = CompressorParams {
                threshold_db: -30.0,
                ratio: 3.0 + b as f64,
                attack_ms: 2.0,
                release_ms: 80.0,
                ..Default::default()
            };
        }
        let noise: Vec<f32> = crate::testsignal::xorshift32(11, 24000).iter().map(|v| (*v * 0.7) as f32).collect();
        let (a, _, _) = run(&p, &noise, &noise, 1);
        let (b, _, _) = run(&p, &noise, &noise, 64);
        let (c, _, _) = run(&p, &noise, &noise, 1000);
        let worst = |x: &[f32], y: &[f32]| x.iter().zip(y).map(|(p, q)| (p - q).abs()).fold(0.0f32, f32::max);
        assert!(worst(&a, &b) <= 1e-6 && worst(&a, &c) <= 1e-6, "{} {}", worst(&a, &b), worst(&a, &c));
        assert!(a.iter().zip(&noise).any(|(x, y)| (x - y).abs() > 0.01), "the compressors did something");
    }

    /// req: MAS-12
    #[test]
    fn stereo_is_linked() {
        let mut p = params(3, &[120.0, 4000.0]);
        p.band[0].comp = CompressorParams {
            threshold_db: -30.0,
            ratio: 4.0,
            attack_ms: 1.0,
            release_ms: 500.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let l = sine(50.0, -6.0, 96000);
        let r: Vec<f32> = sine(50.0, -46.0, 96000);
        let (yl, yr, _) = run(&p, &l, &r, 300);
        let gl = bin(&yl[48000..], 50.0, 48000) / bin(&l[48000..], 50.0, 48000);
        let gr = bin(&yr[48000..], 50.0, 48000) / bin(&r[48000..], 50.0, 48000);
        assert!((20.0 * (gl / gr).log10()).abs() < 0.1, "{gl} {gr}");
        assert!(20.0 * gr.log10() < -5.0, "the quiet channel is ducked by the loud one");
    }
}
