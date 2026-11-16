//! Dynamic EQ (FX-15): one or two bands (peak, low shelf, high shelf) that move only while their detector
//! is above the threshold.
//!
//! **Algorithm.** Per band: a *detector* filters the key (the sidechain of the band, or the signal itself when
//! there is none) with the band's own shape: a constant-0-dB band-pass at `freq_hz` with the band's `q` for
//! `Peak`, a Butterworth low-pass at `freq_hz` for `LowShelf`, a Butterworth high-pass for `HighShelf`. The
//! absolute values of the channels are linked (maximum), then followed by a peak envelope with the band's attack
//! and release (one pole each). The level over the threshold is `over = level_db - threshold_db`; the target gain
//! is `range_db * clamp(over / 6, 0, 1)`: the **full range is reached at 6 dB over the threshold** and the gain is
//! exactly 0 dB at or below it. The gain is smoothed (2 ms one pole) and applied as an RBJ biquad
//! ([`Kind::Peak`], [`Kind::LowShelf`] or [`Kind::HighShelf`]) with that gain. Negative `range_db` cuts, positive
//! lifts. When the smoothed gain is below 1e-4 dB and the target is 0, it snaps to exactly 0 and the band is
//! skipped, so a band that is not triggered leaves the signal bit-exact (FX-24).
//!
//! **Block-size independence.** Envelope and gain run per sample; the filter coefficients are recomputed every
//! [`SUBBLOCK`] samples counted since `reset`, never per call.
//!
//! **Latency.** 0 samples (the detector is not look-ahead).
//!
//! **Properties verified (tests).** Neutral and untriggered bands are identity (<= 1e-6); a 3 kHz band cuts
//! a loud 3 kHz tone by 6 dB (+-0.5) leaving 300 Hz untouched (+-0.1 dB); a lift of +4 dB; with a key the band moves
//! only while the key is loud; block-size invariance (<= 1e-6); bounded output and finite results on full-scale
//! noise with parameters changing every 32 samples. Process does not allocate, lock or perform I/O.

use crate::biquad::{BUTTERWORTH_Q, Biquad, Coeffs, Kind};
use crate::util::{SUBBLOCK, one_pole};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeqShape {
    Peak,
    LowShelf,
    HighShelf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeqBandParams {
    pub on: bool,
    pub shape: DeqShape,
    pub freq_hz: f64,
    /// 0.1..10.
    pub q: f64,
    pub threshold_db: f64,
    /// −12..+6 dB: negative cuts, positive lifts.
    pub range_db: f64,
    pub attack_ms: f64,
    pub release_ms: f64,
}

impl Default for DeqBandParams {
    fn default() -> Self {
        DeqBandParams {
            on: false,
            shape: DeqShape::Peak,
            freq_hz: 3000.0,
            q: 1.0,
            threshold_db: -30.0,
            range_db: 0.0,
            attack_ms: 10.0,
            release_ms: 100.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct DynamicEqParams {
    pub bands: [DeqBandParams; 2],
}

/// Over-threshold distance (dB) at which the band reaches its full range.
const FULL_RANGE_OVER_DB: f64 = 6.0;
const GAIN_SMOOTH_MS: f64 = 2.0;
const SNAP_DB: f64 = 1e-4;
/// Smoothing of frequency and Q of the output filter (per coefficient update).
const TUNE_SMOOTH_MS: f64 = 5.0;

/// Constant 0 dB peak gain band-pass (RBJ cookbook).
fn band_pass(freq: f64, q: f64, rate: f64) -> Coeffs {
    let w0 = 2.0 * PI * freq.clamp(1.0, rate * 0.4999) / rate;
    let (sin, cos) = w0.sin_cos();
    let alpha = sin / (2.0 * q.max(1e-3));
    let a0 = 1.0 + alpha;
    Coeffs { b0: alpha / a0, b1: 0.0, b2: -alpha / a0, a1: -2.0 * cos / a0, a2: (1.0 - alpha) / a0 }
}

#[derive(Clone, Debug)]
struct Band {
    p: DeqBandParams,
    det: Biquad,
    filt: Biquad,
    env: f64,
    gain_db: f64,
    att: f64,
    rel: f64,
    smooth: f64,
    /// Largest-magnitude signed gain since the last `take_gain_db`.
    peak_gain: f64,
    /// Frequency (log2 Hz) and Q (log2) of the output filter, smoothed on the coefficient grid so automation
    /// of the tuning does not make the biquad burst.
    freq_s: f64,
    q_s: f64,
    tune_k: f64,
}

impl Band {
    fn new(rate: f64) -> Self {
        let p = DeqBandParams::default();
        let mut b = Band {
            p,
            det: Biquad::default(),
            filt: Biquad::default(),
            env: 0.0,
            gain_db: 0.0,
            att: 0.0,
            rel: 0.0,
            smooth: one_pole(GAIN_SMOOTH_MS, rate),
            peak_gain: 0.0,
            freq_s: p.freq_hz.log2(),
            q_s: p.q.log2(),
            tune_k: 1.0 - (-(SUBBLOCK as f64) / (TUNE_SMOOTH_MS * 0.001 * rate)).exp(),
        };
        b.set(&p, rate);
        b
    }

    fn set(&mut self, p: &DeqBandParams, rate: f64) {
        let shape_or_tuning_changed = p.shape != self.p.shape || p.freq_hz != self.p.freq_hz || p.q != self.p.q;
        self.p = *p;
        self.att = one_pole(p.attack_ms.max(0.0), rate);
        self.rel = one_pole(p.release_ms.max(0.0), rate);
        if shape_or_tuning_changed {
            self.det.c = match p.shape {
                DeqShape::Peak => band_pass(p.freq_hz, p.q.clamp(0.1, 10.0), rate),
                DeqShape::LowShelf => Coeffs::design(Kind::LowPass, p.freq_hz, 0.0, BUTTERWORTH_Q, rate),
                DeqShape::HighShelf => Coeffs::design(Kind::HighPass, p.freq_hz, 0.0, BUTTERWORTH_Q, rate),
            };
        }
    }

    fn reset(&mut self) {
        self.det.reset();
        self.filt.reset();
        self.env = 0.0;
        self.gain_db = 0.0;
        self.peak_gain = 0.0;
        self.freq_s = self.p.freq_hz.max(1.0).log2();
        self.q_s = self.p.q.clamp(0.1, 10.0).log2();
    }

    /// Updates the envelope with the linked detector level `lvl` (linear, >= 0) and the smoothed gain.
    #[inline]
    fn follow(&mut self, lvl: f64) {
        let c = if lvl > self.env { self.att } else { self.rel };
        self.env += c * (lvl - self.env);
        if self.env < 1e-30 {
            self.env = 0.0;
        }
        let target = if self.p.on {
            let level_db = 20.0 * self.env.max(1e-9).log10();
            let over = level_db - self.p.threshold_db;
            self.p.range_db.clamp(-12.0, 6.0) * (over / FULL_RANGE_OVER_DB).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.gain_db += self.smooth * (target - self.gain_db);
        if target == 0.0 && self.gain_db.abs() < SNAP_DB {
            self.gain_db = 0.0;
        }
        if self.gain_db.abs() > self.peak_gain.abs() {
            self.peak_gain = self.gain_db;
        }
    }

    fn update_coeffs(&mut self, rate: f64) {
        self.freq_s += self.tune_k * (self.p.freq_hz.max(1.0).log2() - self.freq_s);
        self.q_s += self.tune_k * (self.p.q.clamp(0.1, 10.0).log2() - self.q_s);
        self.filt.c = if self.gain_db == 0.0 {
            Coeffs::IDENTITY
        } else {
            let kind = match self.p.shape {
                DeqShape::Peak => Kind::Peak,
                DeqShape::LowShelf => Kind::LowShelf,
                DeqShape::HighShelf => Kind::HighShelf,
            };
            Coeffs::design(kind, self.freq_s.exp2(), self.gain_db, self.q_s.exp2(), rate)
        };
    }
}

pub struct DynamicEq {
    rate: f64,
    bands: [Band; 2],
    /// Samples processed since `reset`: the coefficient grid is anchored here.
    count: usize,
}

impl DynamicEq {
    pub fn new(rate: f64) -> Self {
        DynamicEq { rate, bands: [Band::new(rate), Band::new(rate)], count: 0 }
    }

    pub fn set_params(&mut self, p: &DynamicEqParams) {
        for (b, bp) in self.bands.iter_mut().zip(p.bands.iter()) {
            b.set(bp, self.rate);
        }
    }

    pub fn reset(&mut self) {
        for b in &mut self.bands {
            b.reset();
        }
        self.count = 0;
    }

    /// Deepest cut (dB, ≤ 0) or largest lift taken by any band since the last call, as a signed value of the
    /// largest magnitude.
    pub fn take_gain_db(&mut self) -> [f64; 2] {
        let mut out = [0.0; 2];
        for (o, b) in out.iter_mut().zip(self.bands.iter_mut()) {
            *o = b.peak_gain;
            b.peak_gain = b.gain_db;
        }
        out
    }

    /// 1 or 2 channels in place. `keys[b]`: the sidechain of band `b` (1 or 2 channels), or `None` to listen to
    /// the signal itself.
    pub fn process(&mut self, chans: &mut [&mut [f32]], keys: [Option<&[&[f32]]>; 2]) {
        let nch = chans.len().min(2);
        if nch == 0 {
            return;
        }
        let n = chans[0].len();
        for i in 0..n {
            if self.count.is_multiple_of(SUBBLOCK) {
                for b in &mut self.bands {
                    b.update_coeffs(self.rate);
                }
            }
            self.count = self.count.wrapping_add(1);
            let mut x = [0.0f64; 2];
            for ch in 0..nch {
                x[ch] = f64::from(chans[ch][i]);
            }
            for (bi, b) in self.bands.iter_mut().enumerate() {
                if !b.p.on && b.gain_db == 0.0 {
                    b.env = 0.0;
                    continue;
                }
                let mut lvl = 0.0f64;
                match keys[bi] {
                    // Every channel of the key, also when the signal is mono (a source panned to one side).
                    Some(k) if !k.is_empty() => {
                        for ch in 0..k.len().min(2) {
                            lvl = lvl.max(b.det.tick(ch, f64::from(k[ch][i])).abs());
                        }
                    }
                    _ => {
                        for ch in 0..nch {
                            lvl = lvl.max(b.det.tick(ch, x[ch]).abs());
                        }
                    }
                }
                b.follow(lvl);
                if b.filt.c != Coeffs::IDENTITY {
                    for ch in 0..nch {
                        x[ch] = b.filt.tick(ch, x[ch]);
                    }
                } else {
                    b.filt.reset();
                }
            }
            for ch in 0..nch {
                chans[ch][i] = x[ch] as f32;
            }
        }
        for b in &mut self.bands {
            b.det.flush_state();
            b.filt.flush_state();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testsignal::xorshift32;

    const FS: f64 = 48_000.0;

    fn tone(f: f64, amp: f64, n: usize) -> Vec<f32> {
        (0..n).map(|i| (amp * (2.0 * PI * f * i as f64 / FS).sin()) as f32).collect()
    }

    /// Amplitude of the component at `f` over `buf` (single-bin correlation; `buf` spans whole cycles).
    fn amp_at(buf: &[f32], f: f64) -> f64 {
        let (mut re, mut im) = (0.0, 0.0);
        for (i, &s) in buf.iter().enumerate() {
            let w = 2.0 * PI * f * i as f64 / FS;
            re += f64::from(s) * w.cos();
            im += f64::from(s) * w.sin();
        }
        2.0 * (re * re + im * im).sqrt() / buf.len() as f64
    }

    fn db(x: f64) -> f64 {
        20.0 * x.log10()
    }

    fn band(range: f64, thr: f64) -> DeqBandParams {
        DeqBandParams {
            on: true,
            shape: DeqShape::Peak,
            freq_hz: 3000.0,
            q: 4.0,
            threshold_db: thr,
            range_db: range,
            attack_ms: 2.0,
            release_ms: 20.0,
        }
    }

    fn params(b0: DeqBandParams) -> DynamicEqParams {
        DynamicEqParams { bands: [b0, DeqBandParams::default()] }
    }

    fn run_mono(d: &mut DynamicEq, x: &[f32], key: Option<&[f32]>) -> Vec<f32> {
        let mut y = x.to_vec();
        match key {
            Some(k) => {
                let ks = [k];
                d.process(&mut [&mut y[..]], [Some(&ks[..]), None]);
            }
            None => d.process(&mut [&mut y[..]], [None, None]),
        }
        y
    }

    fn max_diff(a: &[f32], b: &[f32]) -> f64 {
        a.iter().zip(b).map(|(x, y)| f64::from((x - y).abs())).fold(0.0, f64::max)
    }

    /// req: FX-15, FX-24 — a tone below the threshold passes unchanged (the band stays at exactly 0 dB).
    #[test]
    fn below_threshold_is_identity() {
        let mut d = DynamicEq::new(FS);
        d.set_params(&params(band(-6.0, -10.0)));
        let x = tone(3000.0, 0.1, 24_000); // -20 dBFS, 10 dB under the threshold
        let y = run_mono(&mut d, &x, None);
        // 1e-6: the gain is exactly 0 so the filter is skipped; only the f32 round trip remains.
        assert!(max_diff(&x, &y) <= 1e-6);
        assert_eq!(d.take_gain_db(), [0.0, 0.0]);
    }

    /// req: FX-15, FX-24 — 6 dB over the threshold with range −6: the 3 kHz tone drops 6 dB (±0.5) and a 300 Hz tone
    /// mixed in is unchanged (±0.1 dB).
    #[test]
    fn cuts_only_the_band() {
        let mut d = DynamicEq::new(FS);
        d.set_params(&params(band(-6.0, -20.0)));
        let n = 48_000;
        let hi = tone(3000.0, 0.5, n); // -6 dBFS: 14 dB over, beyond the 6 dB of full range
        let lo = tone(300.0, 0.2, n);
        let x: Vec<f32> = hi.iter().zip(&lo).map(|(a, b)| a + b).collect();
        let y = run_mono(&mut d, &x, None);
        let tail = n - 4800; // 0.1 s = whole cycles of both tones, after the attack and the smoothing
        let dh = db(amp_at(&y[tail..], 3000.0)) - db(amp_at(&x[tail..], 3000.0));
        let dl = db(amp_at(&y[tail..], 300.0)) - db(amp_at(&x[tail..], 300.0));
        assert!((dh + 6.0).abs() <= 0.5, "3 kHz change {dh} dB");
        assert!(dl.abs() <= 0.1, "300 Hz change {dl} dB");
        let g = d.take_gain_db();
        assert!((g[0] + 6.0).abs() < 0.1 && g[1] == 0.0, "{g:?}");
    }

    /// req: FX-15, FX-24 — positive range lifts: +4 dB (±0.5) above the threshold.
    #[test]
    fn positive_range_lifts() {
        let mut d = DynamicEq::new(FS);
        let x = tone(3000.0, 0.1, 48_000); // -20 dBFS; the threshold below is 10 dB under it
        let mut p = band(4.0, -30.0);
        p.q = 2.0;
        d.set_params(&params(p));
        let y = run_mono(&mut d, &x, None);
        let dh = db(amp_at(&y[43_200..], 3000.0)) - db(amp_at(&x[43_200..], 3000.0));
        assert!((dh - 4.0).abs() <= 0.5, "lift {dh} dB");
    }

    /// req: FX-15, FX-24 — with a key: a 3 kHz "guitar" is cut only while the key (a separate 3 kHz burst) is loud.
    #[test]
    fn key_drives_the_band() {
        let n = 48_000;
        let guitar = tone(3000.0, 0.3, n);
        let mut key = tone(3000.0, 0.5, n);
        for s in &mut key[n / 2..] {
            *s = 0.0;
        }
        let mut d = DynamicEq::new(FS);
        d.set_params(&params(band(-6.0, -20.0))); // the guitar alone (-10.5 dBFS) is above this threshold
        let y = run_mono(&mut d, &guitar, Some(&key));
        let cut = db(amp_at(&y[n / 2 - 4800..n / 2], 3000.0)) - db(amp_at(&guitar[n / 2 - 4800..n / 2], 3000.0));
        assert!((cut + 6.0).abs() <= 0.5, "cut while the key is loud: {cut} dB");
        // after the burst (release 20 ms + smoothing, 0.15 s are plenty) the band is back at exactly 0 dB
        let after = n / 2 + 7200;
        assert!(max_diff(&guitar[after..], &y[after..]) <= 1e-6);
        // a key that plays on the right only moves the band of a mono signal just the same
        let zero = vec![0.0f32; n];
        d.reset();
        let mut y3 = guitar.clone();
        d.process(&mut [&mut y3[..]], [Some(&[&zero[..], &key[..]]), None]);
        let cut = db(amp_at(&y3[n / 2 - 4800..n / 2], 3000.0)) - db(amp_at(&guitar[n / 2 - 4800..n / 2], 3000.0));
        assert!((cut + 6.0).abs() <= 0.5, "cut with the key on the right: {cut} dB");
        // a silent key never moves the band
        let silent = vec![0.0f32; n];
        d.reset();
        let y2 = run_mono(&mut d, &guitar, Some(&silent));
        assert!(max_diff(&guitar, &y2) <= 1e-6);
        assert_eq!(d.take_gain_db(), [0.0, 0.0]);
    }

    /// req: FX-15, FX-24 — a shelf band: a low shelf triggered by low-frequency content cuts the lows only.
    #[test]
    fn low_shelf_cuts_lows() {
        let mut d = DynamicEq::new(FS);
        let mut p = band(-6.0, -20.0);
        p.shape = DeqShape::LowShelf;
        p.freq_hz = 500.0;
        p.q = 0.707;
        d.set_params(&params(p));
        let n = 48_000;
        let lo = tone(100.0, 0.5, n);
        let hi = tone(5000.0, 0.1, n);
        let x: Vec<f32> = lo.iter().zip(&hi).map(|(a, b)| a + b).collect();
        let y = run_mono(&mut d, &x, None);
        let t = n - 4800;
        let dl = db(amp_at(&y[t..], 100.0)) - db(amp_at(&x[t..], 100.0));
        let dh = db(amp_at(&y[t..], 5000.0)) - db(amp_at(&x[t..], 5000.0));
        assert!((dl + 6.0).abs() <= 0.7, "100 Hz change {dl} dB");
        assert!(dh.abs() <= 0.2, "5 kHz change {dh} dB");
    }

    /// req: FX-24 — neutral: range 0 dB, bands off and a loud signal leave the input unchanged (≤ 1e-6).
    #[test]
    fn neutral_is_identity() {
        let n = 24_000;
        let x: Vec<f32> = xorshift32(5, n).iter().map(|&v| v as f32 * 0.8).collect();
        let mut d = DynamicEq::new(FS);
        d.set_params(&params(band(0.0, -60.0)));
        assert!(max_diff(&x, &run_mono(&mut d, &x, None)) <= 1e-6);
        let mut off = band(-6.0, -60.0);
        off.on = false;
        d.reset();
        d.set_params(&params(off));
        assert!(max_diff(&x, &run_mono(&mut d, &x, None)) <= 1e-6);
    }

    fn noise_stereo(n: usize) -> [Vec<f32>; 2] {
        [xorshift32(11, n).iter().map(|&v| v as f32).collect(), xorshift32(12, n).iter().map(|&v| v as f32).collect()]
    }

    fn run_chunks(p: &DynamicEqParams, x: &[Vec<f32>; 2], chunk: usize) -> [Vec<f32>; 2] {
        let mut d = DynamicEq::new(FS);
        d.set_params(p);
        let mut y = x.clone();
        let n = y[0].len();
        let mut i = 0;
        while i < n {
            let e = (i + chunk).min(n);
            let (l, r) = y.split_at_mut(1);
            d.process(&mut [&mut l[0][i..e], &mut r[0][i..e]], [None, None]);
            i = e;
        }
        y
    }

    /// req: FX-24, ENG-4 — the output does not depend on the block size (chunks of 32, 1000 and 7).
    #[test]
    fn block_size_independent() {
        let mut p = params(band(-9.0, -24.0));
        p.bands[1] = band(5.0, -30.0);
        p.bands[1].shape = DeqShape::HighShelf;
        p.bands[1].freq_hz = 6000.0;
        let x = noise_stereo(20_000);
        let a = run_chunks(&p, &x, 32);
        for chunk in [1000, 7] {
            let b = run_chunks(&p, &x, chunk);
            for ch in 0..2 {
                assert!(max_diff(&a[ch], &b[ch]) <= 1e-6, "chunk {chunk} ch {ch}");
            }
        }
    }

    /// req: FX-24 — full-scale noise through extreme settings (both bands at +6 dB, Q 10, instant times) stays finite and
    /// bounded: a +6 dB biquad has |H| ≤ 2 and the impulse response sum stays under 4 here, two bands in series
    /// are allowed a generous bound of 16.
    #[test]
    fn bounded_on_noise() {
        let mut b = band(6.0, -90.0);
        b.q = 10.0;
        b.attack_ms = 0.0;
        b.release_ms = 0.0;
        let mut p = params(b);
        p.bands[1] = b;
        p.bands[1].freq_hz = 200.0;
        p.bands[1].shape = DeqShape::LowShelf;
        let x = noise_stereo(96_000);
        let y = run_chunks(&p, &x, 64);
        for ch in 0..2 {
            assert!(y[ch].iter().all(|v| v.is_finite() && v.abs() <= 16.0));
        }
    }

    /// req: FX-24, AUT-6 — parameters changing every 32 samples on noise keep the output finite and bounded
    /// (64: far above the +12 dB of two series bands; the tuning smoothing keeps the biquads from bursting).
    #[test]
    fn stable_with_changing_params() {
        let mut d = DynamicEq::new(FS);
        let x = noise_stereo(48_000 * 3);
        let r = xorshift32(99, 20_000);
        let mut y = x.clone();
        let mut k = 0;
        for start in (0..y[0].len()).step_by(32) {
            let mut bp = [DeqBandParams::default(); 2];
            for bnd in &mut bp {
                let u = |j: usize| (r[(k + j) % r.len()] + 1.0) * 0.5;
                bnd.on = u(0) > 0.1;
                bnd.shape = match (u(1) * 3.0) as usize {
                    0 => DeqShape::Peak,
                    1 => DeqShape::LowShelf,
                    _ => DeqShape::HighShelf,
                };
                bnd.freq_hz = 50.0 * 400f64.powf(u(2));
                bnd.q = 0.1 + 9.9 * u(3);
                bnd.threshold_db = -60.0 + 50.0 * u(4);
                bnd.range_db = -12.0 + 18.0 * u(5);
                bnd.attack_ms = 20.0 * u(6);
                bnd.release_ms = 200.0 * u(7);
                k += 8;
            }
            d.set_params(&DynamicEqParams { bands: bp });
            let e = (start + 32).min(y[0].len());
            let (l, rr) = y.split_at_mut(1);
            d.process(&mut [&mut l[0][start..e], &mut rr[0][start..e]], [None, None]);
        }
        for ch in 0..2 {
            let m = y[ch].iter().fold(0.0f32, |a, v| if v.is_finite() { a.max(v.abs()) } else { f32::INFINITY });
            assert!(m <= 64.0, "max {m}");
        }
    }
}
